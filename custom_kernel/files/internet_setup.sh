#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  L99 Kernel Net v2 — Internet Connectivity Setup
#
#  This script does the real work of connecting your kernel to the internet:
#    1. Auto-detects your physical NIC
#    2. Brings up l99net0 virtual NIC
#    3. Creates a bridge OR sets up NAT masquerading
#    4. Configures DNS (resolv.conf)
#    5. Sets a default gateway
#    6. Tests internet connectivity (ping + curl)
#    7. Starts SSH + HTTP server
#
#  Usage:
#    sudo ./internet_setup.sh
#    sudo ./internet_setup.sh --no-bridge   (NAT only, safer)
#    sudo ./internet_setup.sh --teardown
#    sudo ./internet_setup.sh --status
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Colours ───────────────────────────────────────────────────────────────────
R='\033[0;31m' G='\033[0;32m' Y='\033[0;33m'
B='\033[0;34m' C='\033[0;36m' W='\033[1;37m' N='\033[0m'

log()    { echo -e "${G}[✓]${N} $*"; }
warn()   { echo -e "${Y}[!]${N} $*"; }
err()    { echo -e "${R}[✗]${N} $*" >&2; }
step()   { echo -e "\n${W}${B}━━━ $* ━━━${N}"; }
ok()     { echo -e "    ${G}✓${N} $1"; }
fail()   { echo -e "    ${R}✗${N} $1"; }

# ── Config ────────────────────────────────────────────────────────────────────
VNIC="l99net0"
BRIDGE="l99br0"
VNIC_IP="10.99.0.1"
VNIC_CIDR="10.99.0.1/24"
SSH_PORT=22
SSH_ALT_PORT=2222
HTTP_PORT=8080
MODULE="l99_net_core"
USE_BRIDGE=true    # set to false with --no-bridge

# DNS servers (Cloudflare primary, Google backup, OpenDNS tertiary)
DNS1="1.1.1.1"
DNS2="8.8.8.8"
DNS3="208.67.222.222"

# ── Parse args ────────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --no-bridge)  USE_BRIDGE=false ;;
        --teardown)   ACTION="teardown" ;;
        --status)     ACTION="status"   ;;
        *)            ACTION="install"  ;;
    esac
done
ACTION="${ACTION:-install}"

# ── Root check ────────────────────────────────────────────────────────────────
[[ $EUID -ne 0 ]] && { err "Run as root: sudo $0"; exit 1; }

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 0 — Auto-detect physical NIC
# ═════════════════════════════════════════════════════════════════════════════
detect_phy() {
    step "Detecting physical network interface"

    # Get the interface used for the default route (most reliable method)
    PHY=$(ip route show default 2>/dev/null \
          | awk '/default/ {print $5; exit}')

    if [[ -z "$PHY" ]]; then
        # Fallback: find first non-loopback UP interface
        PHY=$(ip link show \
              | awk -F': ' '/^[0-9]+: / && !/lo|l99/ {print $2}' \
              | head -1 | tr -d ' @*')
    fi

    if [[ -z "$PHY" ]]; then
        err "No physical NIC detected. Connect a network cable or enable WiFi."
        err "Available interfaces:"
        ip link show | awk -F': ' '/^[0-9]+:/ {print "  " $2}'
        exit 1
    fi

    # Get current gateway
    GW=$(ip route show default dev "$PHY" 2>/dev/null \
         | awk '{print $3; exit}')
    GW="${GW:-$(ip route show default | awk '{print $3; exit}')}"

    # Get current IP of physical NIC
    PHY_IP=$(ip addr show "$PHY" \
             | awk '/inet / {print $2; exit}')

    log "Physical NIC   : $PHY"
    log "Physical IP    : ${PHY_IP:-DHCP/unknown}"
    log "Default gateway: ${GW:-unknown}"

    export PHY GW PHY_IP
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 1 — Load kernel module
# ═════════════════════════════════════════════════════════════════════════════
load_module() {
    step "Loading kernel module"

    if lsmod | grep -q "^${MODULE}"; then
        warn "Module already loaded"
    else
        if [[ -f "${MODULE}.ko" ]]; then
            insmod "${MODULE}.ko" phy_iface="$PHY" && \
                log "Module loaded with phy_iface=$PHY"
        else
            warn "${MODULE}.ko not found – run 'make' first (continuing)"
            warn "Network config will proceed via userspace only."
        fi
    fi
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 2 — Bring up virtual NIC
# ═════════════════════════════════════════════════════════════════════════════
setup_vnic() {
    step "Configuring virtual NIC ($VNIC)"

    if ! ip link show "$VNIC" &>/dev/null; then
        warn "$VNIC not found (module not loaded?). Creating via tun/tap…"
        # Fallback: create a tun device if the kernel module isn't loaded
        if ! command -v ip &>/dev/null; then
            err "iproute2 not installed"
            exit 1
        fi
        ip tuntap add mode tap name "$VNIC" || true
    fi

    # Remove old IP assignments
    ip addr flush dev "$VNIC" 2>/dev/null || true

    # Assign static IP
    ip addr add "$VNIC_CIDR" dev "$VNIC" 2>/dev/null || true
    ip link set "$VNIC" up

    ok "$VNIC is UP with IP $VNIC_IP"
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 3A — Bridge mode (l99net0 + eth0 → l99br0)
#  Best for: transparent bridging where l99net0 shares eth0's IP
# ═════════════════════════════════════════════════════════════════════════════
setup_bridge() {
    step "Setting up bridge: $BRIDGE ($VNIC + $PHY)"

    # Install bridge utilities
    command -v brctl &>/dev/null || apt-get install -y bridge-utils -qq

    # Destroy old bridge if exists
    if ip link show "$BRIDGE" &>/dev/null; then
        ip link set "$BRIDGE" down 2>/dev/null || true
        brctl delbr "$BRIDGE"   2>/dev/null || true
    fi

    # Create bridge
    brctl addbr "$BRIDGE"
    brctl stp   "$BRIDGE" off      # disable STP for low latency
    brctl setfd "$BRIDGE" 0        # zero forward delay

    # Add physical NIC to bridge (it will lose its IP — bridge takes over)
    ip addr flush dev "$PHY" 2>/dev/null || true
    brctl addif "$BRIDGE" "$PHY"

    # Add virtual NIC to bridge
    brctl addif "$BRIDGE" "$VNIC"

    # Give the bridge the IP
    if [[ -n "$PHY_IP" ]]; then
        ip addr add "$PHY_IP" dev "$BRIDGE" 2>/dev/null || true
    fi
    ip link set "$BRIDGE" up
    ip link set "$PHY"    up
    ip link set "$VNIC"   up

    # Restore default route via bridge
    if [[ -n "$GW" ]]; then
        ip route del default 2>/dev/null || true
        ip route add default via "$GW" dev "$BRIDGE"
        ok "Default route: via $GW dev $BRIDGE"
    fi

    log "Bridge $BRIDGE created: [$PHY + $VNIC]"
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 3B — NAT masquerade mode (safer, no bridge needed)
#  Best for: keeping eth0 config intact, routing l99net0 via NAT
# ═════════════════════════════════════════════════════════════════════════════
setup_nat() {
    step "Configuring NAT (masquerade through $PHY)"

    # Enable IP forwarding (kernel parameter)
    sysctl -w net.ipv4.ip_forward=1              > /dev/null
    sysctl -w net.ipv6.conf.all.forwarding=1     > /dev/null 2>&1 || true
    sysctl -w net.ipv4.conf.all.rp_filter=0      > /dev/null
    sysctl -w net.ipv4.conf."$PHY".rp_filter=0   > /dev/null
    sysctl -w net.ipv4.conf."$VNIC".rp_filter=0  > /dev/null 2>&1 || true
    sysctl -w net.core.rmem_max=16777216         > /dev/null
    sysctl -w net.core.wmem_max=16777216         > /dev/null
    ok "IP forwarding enabled"

    # ── iptables NAT rules ─────────────────────────────────────────
    # Masquerade all traffic from the virtual NIC subnet through the real NIC
    iptables -t nat -F POSTROUTING 2>/dev/null || true
    iptables -t nat -A POSTROUTING -s 10.99.0.0/24 -o "$PHY" -j MASQUERADE
    ok "NAT MASQUERADE: 10.99.0.0/24 → $PHY"

    # Allow forwarding between l99net0 and physical NIC
    iptables -F FORWARD 2>/dev/null || true
    iptables -A FORWARD -i "$VNIC" -o "$PHY"  -j ACCEPT
    iptables -A FORWARD -i "$PHY"  -o "$VNIC" -m conntrack \
             --ctstate ESTABLISHED,RELATED      -j ACCEPT
    ok "iptables FORWARD rules set"

    # Allow all traffic on loopback
    iptables -I INPUT 1 -i lo -j ACCEPT

    # ── IPv6 NDP proxy (if physical NIC has IPv6) ──────────────────
    if [[ -n "$(ip -6 addr show "$PHY" scope global 2>/dev/null)" ]]; then
        sysctl -w net.ipv6.conf."$PHY".proxy_ndp=1   > /dev/null 2>&1 || true
        ip6tables -t nat -A POSTROUTING -o "$PHY" -j MASQUERADE 2>/dev/null || true
        ok "IPv6 masquerade enabled on $PHY"
    fi

    # ── Default route ──────────────────────────────────────────────
    if [[ -n "$GW" ]]; then
        # Keep existing default route; add specific route for virtual subnet
        ip route add 10.99.0.0/24 dev "$VNIC" 2>/dev/null || true
        ok "Route: 10.99.0.0/24 dev $VNIC"
        ok "Default route: via $GW dev $PHY (unchanged)"
    else
        warn "No default gateway detected. Attempting DHCP on $PHY…"
        dhclient "$PHY" -timeout 15 2>/dev/null || \
        dhcpcd  "$PHY"              2>/dev/null || \
            warn "DHCP failed. Set gateway manually: ip route add default via <GW>"
    fi
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 4 — DNS configuration
# ═════════════════════════════════════════════════════════════════════════════
configure_dns() {
    step "Configuring DNS resolvers"

    # Back up resolv.conf
    cp /etc/resolv.conf /etc/resolv.conf.l99bak 2>/dev/null || true

    # Check if systemd-resolved is managing DNS
    if systemctl is-active --quiet systemd-resolved 2>/dev/null; then
        # Use resolvectl for systemd-resolved
        resolvectl dns    "$PHY" "$DNS1" "$DNS2" "$DNS3" 2>/dev/null || true
        resolvectl domain "$PHY" "~."                     2>/dev/null || true
        ok "DNS via systemd-resolved: $DNS1, $DNS2, $DNS3"
    else
        # Direct resolv.conf approach
        cat > /etc/resolv.conf <<DNS
# L99 Kernel Net — DNS configuration
# Generated by internet_setup.sh
nameserver $DNS1
nameserver $DNS2
nameserver $DNS3
options edns0 trust-ad
search local
DNS
        ok "DNS written to /etc/resolv.conf"
    fi

    ok "Resolvers: $DNS1 (Cloudflare), $DNS2 (Google), $DNS3 (OpenDNS)"
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 5 — Firewall: open SSH + HTTP ports, block the rest
# ═════════════════════════════════════════════════════════════════════════════
configure_firewall() {
    step "Configuring firewall"

    # INPUT chain
    iptables -P INPUT DROP
    iptables -F INPUT 2>/dev/null || true

    iptables -A INPUT -i lo          -j ACCEPT
    iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
    iptables -A INPUT -p tcp --dport "$SSH_PORT"     -j ACCEPT
    iptables -A INPUT -p tcp --dport "$SSH_ALT_PORT" -j ACCEPT
    iptables -A INPUT -p tcp --dport "$HTTP_PORT"    -j ACCEPT
    iptables -A INPUT -p icmp                        -j ACCEPT
    iptables -A INPUT -p tcp --dport 443             -j ACCEPT   # HTTPS in
    iptables -A INPUT -p udp --dport 53              -j ACCEPT   # DNS

    # OUTPUT: allow all outbound
    iptables -P OUTPUT ACCEPT

    ok "Firewall: SSH($SSH_PORT/$SSH_ALT_PORT) HTTP($HTTP_PORT) ICMP HTTPS open"
    ok "Firewall: all outbound traffic allowed"
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 6 — Persist settings across reboots
# ═════════════════════════════════════════════════════════════════════════════
persist_settings() {
    step "Persisting network configuration"

    # sysctl persistence
    cat > /etc/sysctl.d/99-l99net.conf <<'SYSCTL'
# L99 Kernel Net — sysctl settings
net.ipv4.ip_forward = 1
net.ipv6.conf.all.forwarding = 1
net.ipv4.conf.all.rp_filter = 0
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_window_scaling = 1
net.ipv4.tcp_timestamps = 1
net.ipv4.tcp_sack = 1
net.ipv4.tcp_congestion_control = bbr
SYSCTL
    sysctl -p /etc/sysctl.d/99-l99net.conf > /dev/null 2>&1 || true
    ok "sysctl settings saved to /etc/sysctl.d/99-l99net.conf"

    # iptables persistence
    if command -v iptables-save &>/dev/null; then
        mkdir -p /etc/iptables
        iptables-save  > /etc/iptables/rules.v4
        ip6tables-save > /etc/iptables/rules.v6 2>/dev/null || true
        ok "iptables rules saved to /etc/iptables/rules.v4"
    fi

    # systemd service for re-applying rules on boot
    cat > /etc/systemd/system/l99net.service <<SERVICE
[Unit]
Description=L99 Kernel Net — Internet & NAT
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/bin/bash $(realpath "$0") --no-bridge
ExecStop=/bin/bash $(realpath "$0") --teardown

[Install]
WantedBy=multi-user.target
SERVICE
    systemctl daemon-reload
    systemctl enable l99net.service 2>/dev/null || true
    ok "systemd service created: l99net.service (auto-start on boot)"
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 7 — Internet connectivity test
# ═════════════════════════════════════════════════════════════════════════════
test_connectivity() {
    step "Testing internet connectivity"

    echo -n "    Pinging 1.1.1.1       … "
    if ping -c 2 -W 3 1.1.1.1 &>/dev/null; then
        echo -e "${G}OK${N}"
    else
        echo -e "${R}FAIL${N}"
        warn "No ICMP reply from 1.1.1.1. Check gateway: $GW"
    fi

    echo -n "    Pinging 8.8.8.8        … "
    if ping -c 2 -W 3 8.8.8.8 &>/dev/null; then
        echo -e "${G}OK${N}"; else echo -e "${R}FAIL${N}"; fi

    echo -n "    DNS lookup (google.com) … "
    if host google.com &>/dev/null 2>&1 || \
       nslookup google.com &>/dev/null 2>&1; then
        echo -e "${G}OK${N}"; else echo -e "${R}FAIL (check DNS)${N}"; fi

    echo -n "    HTTP (http://1.1.1.1)   … "
    if curl -s --max-time 5 http://1.1.1.1 &>/dev/null; then
        echo -e "${G}OK${N}"; else echo -e "${Y}no reply (may be filtered)${N}"; fi

    echo -n "    HTTPS (https://cloudflare.com) … "
    if curl -s --max-time 8 https://www.cloudflare.com &>/dev/null; then
        echo -e "${G}OK${N}"; else echo -e "${Y}timeout${N}"; fi

    echo ""
    log "Connectivity test complete."
}

# ═════════════════════════════════════════════════════════════════════════════
#  STEP 8 — SSH daemon
# ═════════════════════════════════════════════════════════════════════════════
setup_ssh() {
    step "Starting SSH daemon"

    # Generate host keys if missing
    for ktype in rsa ecdsa ed25519; do
        keyfile="/etc/ssh/ssh_host_${ktype}_key"
        [[ -f "$keyfile" ]] || ssh-keygen -t "$ktype" -f "$keyfile" -N "" -q
    done

    # Copy our hardened config
    [[ -f "sshd_config" ]] && cp sshd_config /etc/ssh/sshd_config

    # Validate + start
    if sshd -t -f /etc/ssh/sshd_config 2>/dev/null; then
        systemctl restart sshd 2>/dev/null || \
        systemctl restart ssh  2>/dev/null || \
        (pkill sshd 2>/dev/null; sleep 1; /usr/sbin/sshd -f /etc/ssh/sshd_config)
        ok "sshd running on :$SSH_PORT and :$SSH_ALT_PORT"
    else
        warn "sshd_config invalid — starting with defaults"
        systemctl restart sshd 2>/dev/null || /usr/sbin/sshd
    fi
}

# ═════════════════════════════════════════════════════════════════════════════
#  HTTP server start
# ═════════════════════════════════════════════════════════════════════════════
setup_http() {
    step "Starting userspace HTTP server (port $HTTP_PORT)"

    if [[ -f "l99_httpd.c" ]]; then
        gcc -O2 -Wall -o l99_httpd l99_httpd.c -lpthread 2>/dev/null
        mkdir -p www
        pkill l99_httpd 2>/dev/null || true
        sleep 1
        nohup ./l99_httpd "$HTTP_PORT" ./www > /var/log/l99_httpd.log 2>&1 &
        echo $! > /var/run/l99_httpd.pid
        sleep 1
        kill -0 "$(cat /var/run/l99_httpd.pid 2>/dev/null)" 2>/dev/null && \
            ok "HTTP server on :$HTTP_PORT" || \
            warn "HTTP server may have failed (check /var/log/l99_httpd.log)"
    else
        # Quick Python fallback
        pkill -f "python.*http.server" 2>/dev/null || true
        mkdir -p www
        python3 -m http.server "$HTTP_PORT" --directory www \
            > /var/log/l99_http_py.log 2>&1 &
        ok "HTTP server (python fallback) on :$HTTP_PORT"
    fi
}

# ═════════════════════════════════════════════════════════════════════════════
#  STATUS
# ═════════════════════════════════════════════════════════════════════════════
show_status() {
    echo ""
    echo -e "${W}━━━━━━━━━━━━━━━ L99 Network Status ━━━━━━━━━━━━━━━${N}"

    echo -e "\n${C}── Kernel Module ──${N}"
    lsmod | grep -q "^${MODULE}" && \
        echo -e "  ${G}✓ Loaded${N}" || echo -e "  ${R}✗ Not loaded${N}"

    echo -e "\n${C}── Interfaces ──${N}"
    for iface in "$VNIC" "$BRIDGE" lo; do
        ip addr show "$iface" 2>/dev/null | \
            awk "NR==1{state=\$0} /inet /{print \"  $iface: \" \$2}" || true
    done

    echo -e "\n${C}── Routing ──${N}"
    ip route show | sed 's/^/  /'

    echo -e "\n${C}── NAT Rules ──${N}"
    iptables -t nat -L POSTROUTING -n --line-numbers 2>/dev/null | \
        grep -v "^Chain\|^num\|^$" | sed 's/^/  /' || echo "  (none)"

    echo -e "\n${C}── DNS ──${N}"
    grep nameserver /etc/resolv.conf 2>/dev/null | sed 's/^/  /' || \
        echo "  (not configured)"

    echo -e "\n${C}── Services ──${N}"
    ss -tlnp | grep -E ":($SSH_PORT|$SSH_ALT_PORT|$HTTP_PORT) " | \
        sed 's/^/  /' || echo "  (no services)"

    echo -e "\n${C}── /proc/l99net ──${N}"
    cat /proc/l99net 2>/dev/null | sed 's/^/  /' || \
        echo "  (module not loaded)"

    echo -e "\n${C}── Connectivity ──${N}"
    ping -c 1 -W 2 1.1.1.1 &>/dev/null && \
        echo -e "  ${G}✓ Internet reachable (1.1.1.1)${N}" || \
        echo -e "  ${R}✗ Internet NOT reachable${N}"
    echo ""
}

# ═════════════════════════════════════════════════════════════════════════════
#  TEARDOWN
# ═════════════════════════════════════════════════════════════════════════════
teardown() {
    step "Tearing down L99 network"

    pkill l99_httpd 2>/dev/null || true
    pkill -f "python.*http.server" 2>/dev/null || true

    # Remove NAT rules
    iptables -t nat -F POSTROUTING 2>/dev/null || true
    iptables -F FORWARD            2>/dev/null || true

    # Remove bridge
    if ip link show "$BRIDGE" &>/dev/null; then
        ip link set "$BRIDGE" down
        brctl delif "$BRIDGE" "$PHY"  2>/dev/null || true
        brctl delif "$BRIDGE" "$VNIC" 2>/dev/null || true
        brctl delbr "$BRIDGE"
        # Restore physical NIC IP via DHCP
        dhclient "$PHY" 2>/dev/null || dhcpcd "$PHY" 2>/dev/null || true
        log "Bridge $BRIDGE removed, DHCP re-applied on $PHY"
    fi

    # Unload module
    if lsmod | grep -q "^${MODULE}"; then
        rmmod "$MODULE" && log "Module unloaded"
    fi

    # Restore resolv.conf
    [[ -f /etc/resolv.conf.l99bak ]] && \
        mv /etc/resolv.conf.l99bak /etc/resolv.conf && \
        log "resolv.conf restored"

    log "Teardown complete"
}

# ═════════════════════════════════════════════════════════════════════════════
#  MAIN
# ═════════════════════════════════════════════════════════════════════════════

case "$ACTION" in

    status)
        detect_phy
        show_status
        ;;

    teardown)
        detect_phy
        teardown
        ;;

    install)
        detect_phy
        load_module
        setup_vnic
        if $USE_BRIDGE; then
            setup_bridge || { warn "Bridge failed, falling back to NAT"; setup_nat; }
        else
            setup_nat
        fi
        configure_dns
        configure_firewall
        persist_settings
        test_connectivity
        setup_ssh
        setup_http

        echo ""
        echo -e "${W}╔══════════════════════════════════════════════════════╗${N}"
        echo -e "${W}║   L99 Kernel Net v2 — INTERNET ONLINE               ║${N}"
        echo -e "${W}╠══════════════════════════════════════════════════════╣${N}"
        echo -e "${W}║  Physical NIC : ${C}${PHY:-?}${W}                                ║${N}"
        echo -e "${W}║  Virtual NIC  : ${C}${VNIC} (${VNIC_IP})${W}              ║${N}"
        echo -e "${W}║  Mode         : ${C}$(${USE_BRIDGE} && echo "Bridge" || echo "NAT Masquerade")${W}                         ║${N}"
        echo -e "${W}║  DNS          : ${C}${DNS1}, ${DNS2}${W}                ║${N}"
        echo -e "${W}║  SSH          : ${C}0.0.0.0:${SSH_PORT} / :${SSH_ALT_PORT}${W}                       ║${N}"
        echo -e "${W}║  HTTP status  : ${C}http://localhost:${HTTP_PORT}/status${W}          ║${N}"
        echo -e "${W}║  Kernel proc  : ${C}cat /proc/l99net${W}                      ║${N}"
        echo -e "${W}╚══════════════════════════════════════════════════════╝${N}"
        echo ""
        ;;
esac
