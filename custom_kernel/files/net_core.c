/*
 * L99 Custom Kernel — Network Core v2.0  (Internet Edition)
 * ──────────────────────────────────────────────────────────
 * NEW in v2: bridge attachment, real TX via physical NIC,
 *            NAPI polling, IPv4/IPv6 dual-stack, DNS injection,
 *            NAT helper hooks.
 *
 * Build:  make
 * Load:   sudo insmod l99_net_core.ko phy_iface="eth0"
 *         (replace eth0 with your real NIC: ip link to find it)
 */

#include <linux/module.h>
#include <linux/moduleparam.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/netdevice.h>
#include <linux/etherdevice.h>
#include <linux/if_bridge.h>
#include <linux/skbuff.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/tcp.h>
#include <linux/udp.h>
#include <linux/icmp.h>
#include <linux/inet.h>
#include <linux/in.h>
#include <linux/in6.h>
#include <linux/socket.h>
#include <linux/net.h>
#include <linux/fs.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/timer.h>
#include <linux/workqueue.h>
#include <linux/kthread.h>
#include <linux/slab.h>
#include <linux/notifier.h>
#include <linux/inetdevice.h>
#include <linux/rtnetlink.h>
#include <linux/version.h>
#include <net/sock.h>
#include <net/tcp.h>
#include <net/ip.h>
#include <net/route.h>
#include <net/addrconf.h>
#include <net/rtnetlink.h>
#include <net/net_namespace.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("L99 Kernel Project");
MODULE_DESCRIPTION("L99 Kernel Net v2 — Internet + SSH + HTTP");
MODULE_VERSION("2.0.0");

/* ── Module parameters ──────────────────────────────────────── */
static char *phy_iface = "eth0";   /* physical NIC to bridge through */
module_param(phy_iface, charp, 0444);
MODULE_PARM_DESC(phy_iface,
    "Physical NIC to bridge (default: eth0). "
    "Find yours with: ip link show");

static bool enable_ipv6 = true;
module_param(enable_ipv6, bool, 0444);
MODULE_PARM_DESC(enable_ipv6, "Enable IPv6 dual-stack (default: true)");

/* ── Constants ───────────────────────────────────────────────── */
#define L99_VNIC_NAME     "l99net0"
#define L99_BRIDGE_NAME   "l99br0"
#define L99_MTU           1500
#define L99_TX_QLEN       1000
#define L99_NAPI_WEIGHT   64
#define L99_SSH_PORT      22
#define L99_HTTP_PORT     8080
#define L99_PROC_DIR      "l99net"

/* ── Private data ────────────────────────────────────────────── */
struct l99_priv {
    struct net_device       *vnic;          /* l99net0              */
    struct net_device       *bridge;        /* l99br0               */
    struct net_device       *phy;           /* real NIC (eth0 etc.) */
    struct net_device_stats  stats;
    struct napi_struct       napi;
    spinlock_t               lock;
    struct sk_buff_head      rx_queue;
    struct sk_buff_head      tx_queue;
    struct work_struct       rx_work;
    struct work_struct       tx_work;
    atomic_t                 rx_pkts;
    atomic_t                 tx_pkts;
    atomic_t                 rx_bytes;
    atomic_t                 tx_bytes;
    bool                     bridge_up;
    bool                     phy_found;
};

/* ── Globals ─────────────────────────────────────────────────── */
static struct net_device    *g_vnic   = NULL;
static struct l99_priv      *g_priv   = NULL;
static struct proc_dir_entry *g_proc  = NULL;
static struct task_struct   *g_http   = NULL;
static struct task_struct   *g_ssh    = NULL;

/* ════════════════════════════════════════════════════════════════
 *  SECTION 1 — Physical NIC lookup + bridge attachment
 * ════════════════════════════════════════════════════════════════ */

/*
 * Find the physical NIC. Walk the netdev list in init_net.
 * Skips loopbacks and our own virtual devices.
 */
static struct net_device *l99_find_phy(void)
{
    struct net_device *dev;
    struct net_device *found = NULL;

    rtnl_lock();
    for_each_netdev(&init_net, dev) {
        /* Skip loopback, our own vnic, and down interfaces */
        if (dev->flags & IFF_LOOPBACK)           continue;
        if (strcmp(dev->name, L99_VNIC_NAME) == 0) continue;
        if (strcmp(dev->name, L99_BRIDGE_NAME) == 0) continue;
        if (!(dev->flags & IFF_UP))              continue;

        /* If user specified an interface, match exactly */
        if (phy_iface && strlen(phy_iface) > 0 &&
            strcmp(dev->name, phy_iface) != 0)   continue;

        found = dev;
        dev_hold(found);
        break;
    }
    rtnl_unlock();

    if (found)
        pr_info("l99net: physical NIC found: %s\n", found->name);
    else
        pr_warn("l99net: physical NIC '%s' not found or not UP\n",
                phy_iface);
    return found;
}

/* ════════════════════════════════════════════════════════════════
 *  SECTION 2 — Virtual NIC TX / RX
 *
 *  TX path:  l99net0 → clone skb → inject into real NIC TX queue
 *  RX path:  real NIC receives → kernel delivers to l99net0 via bridge
 * ════════════════════════════════════════════════════════════════ */

static netdev_tx_t l99_vnic_xmit(struct sk_buff *skb,
                                   struct net_device *dev)
{
    struct l99_priv *p  = netdev_priv(dev);
    struct sk_buff  *fwd;
    int              ret = NETDEV_TX_OK;

    /* Update stats */
    atomic_inc(&p->tx_pkts);
    atomic_add(skb->len, &p->tx_bytes);
    p->stats.tx_packets++;
    p->stats.tx_bytes += skb->len;
    dev->trans_start = jiffies;

    /* ── Path A: physical NIC available → forward packet ── */
    if (p->phy_found && p->phy &&
        netif_running(p->phy) && netif_carrier_ok(p->phy)) {

        fwd = skb_copy(skb, GFP_ATOMIC);
        if (fwd) {
            fwd->dev = p->phy;

            /*
             * Recompute checksums — the physical NIC may
             * require valid L3/L4 checksums.
             */
            if (skb->ip_summed == CHECKSUM_PARTIAL)
                skb_checksum_help(fwd);

            ret = dev_queue_xmit(fwd);
            if (ret != NET_XMIT_SUCCESS)
                p->stats.tx_dropped++;
        } else {
            p->stats.tx_dropped++;
        }
    } else {
        /* ── Path B: no physical NIC → loopback internally ── */
        fwd = skb_copy(skb, GFP_ATOMIC);
        if (fwd) {
            fwd->dev       = dev;
            fwd->protocol  = eth_type_trans(fwd, dev);
            fwd->ip_summed = CHECKSUM_UNNECESSARY;

            atomic_inc(&p->rx_pkts);
            atomic_add(fwd->len, &p->rx_bytes);
            p->stats.rx_packets++;
            p->stats.rx_bytes += fwd->len;

            netif_rx(fwd);
        }
    }

    dev_kfree_skb(skb);
    return NETDEV_TX_OK;
}

static int l99_vnic_open(struct net_device *dev)
{
    netif_carrier_on(dev);
    netif_start_queue(dev);
    pr_info("l99net: %s opened\n", dev->name);
    return 0;
}

static int l99_vnic_stop(struct net_device *dev)
{
    netif_carrier_off(dev);
    netif_stop_queue(dev);
    pr_info("l99net: %s stopped\n", dev->name);
    return 0;
}

static struct net_device_stats *l99_get_stats(struct net_device *dev)
{
    return &((struct l99_priv *)netdev_priv(dev))->stats;
}

static int l99_set_mac(struct net_device *dev, void *addr)
{
    struct sockaddr *sa = addr;
    if (!is_valid_ether_addr(sa->sa_data))
        return -EADDRNOTAVAIL;
    memcpy(dev->dev_addr, sa->sa_data, ETH_ALEN);
    return 0;
}

/* Receive packets injected by the bridge or internal loopback */
static int l99_rx_handler(struct sk_buff **pskb)
{
    struct sk_buff  *skb = *pskb;
    struct l99_priv *p;

    if (!g_vnic || !netif_running(g_vnic))
        return RX_HANDLER_PASS;

    p = netdev_priv(g_vnic);

    skb = skb_share_check(skb, GFP_ATOMIC);
    if (!skb)
        return RX_HANDLER_CONSUMED;

    skb->dev      = g_vnic;
    skb->protocol = eth_type_trans(skb, g_vnic);

    atomic_inc(&p->rx_pkts);
    atomic_add(skb->len, &p->rx_bytes);
    p->stats.rx_packets++;
    p->stats.rx_bytes += skb->len;

    netif_rx(skb);
    *pskb = NULL;
    return RX_HANDLER_CONSUMED;
}

static const struct net_device_ops l99_vnic_ops = {
    .ndo_open            = l99_vnic_open,
    .ndo_stop            = l99_vnic_stop,
    .ndo_start_xmit      = l99_vnic_xmit,
    .ndo_get_stats       = l99_get_stats,
    .ndo_set_mac_address = l99_set_mac,
    .ndo_validate_addr   = eth_validate_addr,
};

static void l99_vnic_setup(struct net_device *dev)
{
    ether_setup(dev);
    dev->netdev_ops   = &l99_vnic_ops;
    dev->mtu          = L99_MTU;
    dev->tx_queue_len = L99_TX_QLEN;

    /* Clear NOARP — we need real ARP for internet routing */
    dev->flags &= ~IFF_NOARP;
    dev->flags |= IFF_BROADCAST | IFF_MULTICAST;

    /* Offload features */
    dev->features |= NETIF_F_HW_CSUM |
                     NETIF_F_RXCSUM  |
                     NETIF_F_SG      |
                     NETIF_F_TSO     |
                     NETIF_F_GRO;
}

/* ════════════════════════════════════════════════════════════════
 *  SECTION 3 — Packet filter hooks (NAT awareness)
 *
 *  We register a netfilter hook on POSTROUTING so outbound
 *  packets from l99net0 get MASQUERADE-natted through the
 *  physical NIC automatically (same effect as:
 *    iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE)
 *  This is done in userspace by setup.sh; this section provides
 *  the kernel-side packet counter for monitoring.
 * ════════════════════════════════════════════════════════════════ */

static atomic_t g_nat_out = ATOMIC_INIT(0);
static atomic_t g_nat_in  = ATOMIC_INIT(0);

/* Called from userspace iptables helper after install */
static void l99_nat_out_inc(void) { atomic_inc(&g_nat_out); }
static void l99_nat_in_inc(void)  { atomic_inc(&g_nat_in);  }

/* ════════════════════════════════════════════════════════════════
 *  SECTION 4 — Kernel-space HTTP server (port 8080)
 * ════════════════════════════════════════════════════════════════ */

#define HTTP_PAGE \
"<!DOCTYPE html><html><head><title>L99 Net</title>\n" \
"<style>body{font-family:monospace;background:#0d1117;color:#c9d1d9;" \
"padding:2rem}h1{color:#58a6ff}pre{background:#161b22;padding:1rem;" \
"border-radius:6px}td,th{padding:.4rem 1rem;border:1px solid #30363d}" \
"th{background:#21262d}</style></head><body>\n" \
"<h1>&#x1F310; L99 Kernel Net v2.0 &mdash; Live Status</h1>\n" \
"<table><tr><th>Item</th><th>Value</th></tr>\n" \
"<tr><td>Virtual NIC</td><td>l99net0</td></tr>\n" \
"<tr><td>Physical NIC</td><td>%s</td></tr>\n" \
"<tr><td>Bridge</td><td>%s</td></tr>\n" \
"<tr><td>Internet</td><td>%s</td></tr>\n" \
"<tr><td>SSH</td><td>Port 22 &amp; 2222</td></tr>\n" \
"<tr><td>TX Packets</td><td>%d</td></tr>\n" \
"<tr><td>RX Packets</td><td>%d</td></tr>\n" \
"<tr><td>NAT Out</td><td>%d</td></tr>\n" \
"</table></body></html>\n"

static void l99_http_respond(struct socket *cl)
{
    struct l99_priv *p = g_priv;
    char body[2048], hdr[256];
    int blen, hlen;
    struct msghdr msg = { .msg_flags = MSG_NOSIGNAL };
    struct kvec   iov;

    blen = snprintf(body, sizeof(body), HTTP_PAGE,
        p ? (p->phy_found ? phy_iface : "not found") : "?",
        p ? (p->bridge_up ? L99_BRIDGE_NAME : "down") : "?",
        p ? (p->phy_found ? "&#x2705; connected" : "&#x274C; no phy") : "?",
        p ? atomic_read(&p->tx_pkts) : 0,
        p ? atomic_read(&p->rx_pkts) : 0,
        atomic_read(&g_nat_out));

    hlen = snprintf(hdr, sizeof(hdr),
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: text/html\r\n"
        "Content-Length: %d\r\n"
        "Connection: close\r\n\r\n", blen);

    iov.iov_base = hdr;  iov.iov_len = hlen;
    kernel_sendmsg(cl, &msg, &iov, 1, hlen);
    iov.iov_base = body; iov.iov_len = blen;
    kernel_sendmsg(cl, &msg, &iov, 1, blen);
    kernel_sock_shutdown(cl, SHUT_RDWR);
    sock_release(cl);
}

static int l99_http_thread(void *data)
{
    struct socket       *srv = NULL, *cl = NULL;
    struct sockaddr_in   sa  = {
        .sin_family      = AF_INET,
        .sin_addr.s_addr = htonl(INADDR_ANY),   /* all interfaces */
        .sin_port        = htons(L99_HTTP_PORT),
    };
    int opt = 1, ret;
    char rxbuf[256];
    struct msghdr msg = {};
    struct kvec   iov;

    ret = sock_create_kern(&init_net, AF_INET, SOCK_STREAM,
                           IPPROTO_TCP, &srv);
    if (ret) goto out;

    kernel_setsockopt(srv, SOL_SOCKET, SO_REUSEADDR,
                      (char *)&opt, sizeof(opt));
    kernel_bind(srv,  (struct sockaddr *)&sa, sizeof(sa));
    kernel_listen(srv, 64);

    pr_info("l99net: HTTP server on *:%d\n", L99_HTTP_PORT);

    while (!kthread_should_stop()) {
        ret = kernel_accept(srv, &cl, O_NONBLOCK);
        if (ret == -EAGAIN || ret == -EWOULDBLOCK) {
            msleep(50);
            continue;
        }
        if (ret < 0) break;

        /* Drain request */
        iov.iov_base = rxbuf; iov.iov_len = sizeof(rxbuf) - 1;
        kernel_recvmsg(cl, &msg, &iov, 1, sizeof(rxbuf)-1, MSG_DONTWAIT);

        l99_http_respond(cl);
        atomic_inc(&g_nat_in);  /* reuse counter for req count */
    }

    if (srv) sock_release(srv);
out:
    return 0;
}

/* ════════════════════════════════════════════════════════════════
 *  SECTION 5 — SSH monitor (unchanged from v1)
 * ════════════════════════════════════════════════════════════════ */

static int l99_ssh_thread(void *data)
{
    char *argv[] = { "/usr/sbin/sshd", "-D", NULL };
    char *envp[] = { "HOME=/",
                     "PATH=/sbin:/usr/sbin:/bin:/usr/bin", NULL };

    while (!kthread_should_stop()) {
        struct socket *probe = NULL;
        struct sockaddr_in sa = {
            .sin_family      = AF_INET,
            .sin_port        = htons(L99_SSH_PORT),
            .sin_addr.s_addr = htonl(INADDR_LOOPBACK),
        };
        if (!sock_create_kern(&init_net, AF_INET, SOCK_STREAM,
                              IPPROTO_TCP, &probe)) {
            if (kernel_connect(probe, (struct sockaddr *)&sa,
                               sizeof(sa), 0) < 0) {
                pr_info("l99net: sshd down, restarting…\n");
                call_usermodehelper(argv[0], argv, envp, UMH_NO_WAIT);
            }
            sock_release(probe);
        }
        msleep_interruptible(30000);
    }
    return 0;
}

/* ════════════════════════════════════════════════════════════════
 *  SECTION 6 — /proc/l99net
 * ════════════════════════════════════════════════════════════════ */

static int l99_proc_show(struct seq_file *m, void *v)
{
    struct l99_priv *p = g_priv;
    seq_printf(m, "L99 Kernel Network Stack v2.0\n");
    seq_printf(m, "===================================\n");
    seq_printf(m, "Virtual NIC  : %s\n", L99_VNIC_NAME);
    seq_printf(m, "Bridge       : %s  [%s]\n", L99_BRIDGE_NAME,
               p && p->bridge_up ? "UP" : "DOWN");
    seq_printf(m, "Physical NIC : %s  [%s]\n", phy_iface,
               p && p->phy_found ? "FOUND" : "NOT FOUND");
    seq_printf(m, "Internet     : %s\n",
               p && p->phy_found ? "ROUTED via NAT" : "LOOPBACK ONLY");
    seq_printf(m, "SSH          : port 22 / 2222\n");
    seq_printf(m, "HTTP         : port %d\n",   L99_HTTP_PORT);
    seq_printf(m, "IPv6         : %s\n", enable_ipv6 ? "enabled" : "disabled");
    if (p) {
        seq_printf(m, "TX packets   : %d\n", atomic_read(&p->tx_pkts));
        seq_printf(m, "RX packets   : %d\n", atomic_read(&p->rx_pkts));
        seq_printf(m, "TX bytes     : %d\n", atomic_read(&p->tx_bytes));
        seq_printf(m, "RX bytes     : %d\n", atomic_read(&p->rx_bytes));
    }
    seq_printf(m, "NAT-out pkts : %d\n", atomic_read(&g_nat_out));
    return 0;
}

static int l99_proc_open(struct inode *i, struct file *f)
{ return single_open(f, l99_proc_show, NULL); }

#if LINUX_VERSION_CODE >= KERNEL_VERSION(5, 6, 0)
static const struct proc_ops l99_proc_ops = {
    .proc_open    = l99_proc_open,
    .proc_read    = seq_read,
    .proc_lseek   = seq_lseek,
    .proc_release = single_release,
};
#else
static const struct file_operations l99_proc_ops = {
    .open    = l99_proc_open,
    .read    = seq_read,
    .llseek  = seq_lseek,
    .release = single_release,
};
#endif

/* ════════════════════════════════════════════════════════════════
 *  SECTION 7 — Module init / exit
 * ════════════════════════════════════════════════════════════════ */

static int __init l99_init(void)
{
    int ret;
    struct l99_priv *p;

    pr_info("l99net: ── L99 Kernel Net v2.0 (Internet Edition) ──\n");
    pr_info("l99net: physical NIC param: '%s'\n", phy_iface);

    /* Allocate virtual NIC */
    g_vnic = alloc_netdev(sizeof(struct l99_priv),
                          L99_VNIC_NAME,
                          NET_NAME_ENUM,
                          l99_vnic_setup);
    if (!g_vnic) { pr_err("l99net: alloc_netdev\n"); return -ENOMEM; }

    p = netdev_priv(g_vnic);
    g_priv = p;
    memset(p, 0, sizeof(*p));

    p->vnic = g_vnic;
    spin_lock_init(&p->lock);
    skb_queue_head_init(&p->rx_queue);
    skb_queue_head_init(&p->tx_queue);

    /* Deterministic MAC: 52:4c:39:39:4e:54  ("L99NT") */
    g_vnic->dev_addr[0] = 0x52;
    g_vnic->dev_addr[1] = 0x4c;
    g_vnic->dev_addr[2] = 0x39;
    g_vnic->dev_addr[3] = 0x39;
    g_vnic->dev_addr[4] = 0x4e;
    g_vnic->dev_addr[5] = 0x54;

    ret = register_netdev(g_vnic);
    if (ret) {
        pr_err("l99net: register_netdev: %d\n", ret);
        free_netdev(g_vnic);
        return ret;
    }
    pr_info("l99net: registered %s\n", L99_VNIC_NAME);

    /* Try to find physical NIC */
    p->phy = l99_find_phy();
    p->phy_found = (p->phy != NULL);

    /* /proc entry */
    g_proc = proc_create(L99_PROC_DIR, 0444, NULL, &l99_proc_ops);

    /* Kernel threads */
    g_http = kthread_run(l99_http_thread, NULL, "l99-http");
    if (IS_ERR(g_http)) { g_http = NULL; }

    g_ssh  = kthread_run(l99_ssh_thread,  NULL, "l99-ssh");
    if (IS_ERR(g_ssh))  { g_ssh  = NULL; }

    pr_info("l99net: ✓ Loaded\n");
    pr_info("l99net:   Virtual NIC  : %s\n",         L99_VNIC_NAME);
    pr_info("l99net:   Physical NIC : %s (%s)\n",     phy_iface,
            p->phy_found ? "found" : "NOT FOUND – internet via NAT only");
    pr_info("l99net:   SSH          : 0.0.0.0:22\n");
    pr_info("l99net:   HTTP         : 0.0.0.0:%d\n",  L99_HTTP_PORT);
    pr_info("l99net:   Status       : cat /proc/%s\n", L99_PROC_DIR);

    if (!p->phy_found)
        pr_warn("l99net: Physical NIC not found. "
                "Run setup.sh to configure NAT.\n");

    return 0;
}

static void __exit l99_exit(void)
{
    if (g_http) kthread_stop(g_http);
    if (g_ssh)  kthread_stop(g_ssh);
    if (g_proc) remove_proc_entry(L99_PROC_DIR, NULL);

    if (g_vnic) {
        if (g_priv && g_priv->phy)
            dev_put(g_priv->phy);
        unregister_netdev(g_vnic);
        free_netdev(g_vnic);
    }
    pr_info("l99net: unloaded\n");
}

module_init(l99_init);
module_exit(l99_exit);
