#include "tcp.h"
#include "ip.h"
#include "netdev.h"
#include "log.h"
#include "libc/string.h"
#include "libc/stdio.h"
#include "../drivers/timer.h"

/* Global TCP State (simplified for single connection) */
static volatile enum tcp_state g_tcp_state = TCP_CLOSED;
static uint32_t g_remote_ip = 0;
static uint16_t g_remote_port = 0;
static uint32_t g_local_seq = 0;
static uint32_t g_remote_ack = 0;

void tcp_init(void) {
    g_tcp_state = TCP_CLOSED;
    // kprint("[TCP] Initialized.\n");
}

/* Checksum Algorithm */
uint16_t tcp_calculate_checksum(void *packet, uint16_t len, uint32_t src_ip, uint32_t dst_ip) {
    uint32_t sum = 0;
    struct tcp_pseudo_header ph;
    
    ph.src_ip = src_ip;
    ph.dst_ip = dst_ip;
    ph.zeros = 0;
    ph.protocol = 6; /* TCP */
    ph.tcp_len = __builtin_bswap16(len);
    
    /* Sum Pseudo Header */
    uint16_t *p = (uint16_t *)&ph;
    for (size_t i = 0; i < sizeof(struct tcp_pseudo_header) / 2; i++) {
        sum += p[i];
    }
    
    /* Sum TCP Segment */
    p = (uint16_t *)packet;
    for (int i = 0; i < len / 2; i++) {
        sum += p[i];
    }
    
    if (len & 1) {
        sum += ((uint8_t *)packet)[len - 1]; /* Padding */
    }
    
    while (sum >> 16) {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    
    return ~sum;
}

/* Send a TCP Segment */
void tcp_send_packet(uint32_t dst_ip, uint16_t dst_port, uint8_t flags, const void *payload, uint16_t payload_len) {
    uint16_t total_len = sizeof(struct tcp_header) + payload_len;
    uint8_t buffer[1500];
    
    struct tcp_header *hdr = (struct tcp_header *)buffer;
    memset(hdr, 0, sizeof(struct tcp_header));
    
    hdr->src_port = __builtin_bswap16(55555); /* Ephemeral Port */
    hdr->dst_port = __builtin_bswap16(dst_port);
    hdr->seq = __builtin_bswap32(g_local_seq);
    hdr->ack = __builtin_bswap32(g_remote_ack);
    hdr->offset_reserved = (sizeof(struct tcp_header) / 4) << 4;
    hdr->flags = flags;
    hdr->window_size = __builtin_bswap16(8192); /* 8KB Window */
    hdr->checksum = 0;
    hdr->urgent_ptr = 0;
    
    if (payload && payload_len > 0) {
        memcpy(buffer + sizeof(struct tcp_header), payload, payload_len);
    }
    
    /* Calculate Checksum */
    /* Need Local IP - simplified, assume 10.0.2.15 */
    uint32_t src_ip = 0x0F02000A; /* 10.0.2.15 */
    hdr->checksum = tcp_calculate_checksum(buffer, total_len, src_ip, dst_ip);
    
    /* Send via IP */
    ip_send(dst_ip, 6, buffer, total_len);
    
    /* Advance Sequence if consuming (SYN/FIN count as 1, Data counts as len) */
    if (flags & (TCP_FLAG_SYN | TCP_FLAG_FIN)) g_local_seq++;
    g_local_seq += payload_len;
}

/* Handle Incoming TCP */
void tcp_handler(void *packet, uint16_t len, uint32_t src_ip) {
    if (len < sizeof(struct tcp_header)) return;
    
    struct tcp_header *hdr = (struct tcp_header *)packet;
    
    // kprint("[TCP] Packet Received.\n");
    
    uint32_t seq = __builtin_bswap32(hdr->seq);
    uint32_t ack_num = __builtin_bswap32(hdr->ack);
    (void)ack_num;
    uint8_t flags = hdr->flags;
    
    /* State Machine Logic */
    if (g_tcp_state == TCP_SYN_SENT) {
        if ((flags & TCP_FLAG_SYN) && (flags & TCP_FLAG_ACK)) {
            // kprint("[TCP] SYN-ACK Received!\n");
            
            g_remote_ack = seq + 1;
            g_tcp_state = TCP_ESTABLISHED;
            
            /* Send ACK */
            tcp_send_packet(src_ip, __builtin_bswap16(hdr->src_port), TCP_FLAG_ACK, NULL, 0);
            kprint("[TCP] Connection Established.\n");
        }
    } else if (g_tcp_state == TCP_ESTABLISHED) {
        if (flags & TCP_FLAG_ACK) {
             /* Data? */
        }
        if (flags & TCP_FLAG_FIN) {
             /* Close */
             g_remote_ack = seq + 1;
             tcp_send_packet(src_ip, __builtin_bswap16(hdr->src_port), TCP_FLAG_ACK | TCP_FLAG_FIN, NULL, 0);
             g_tcp_state = TCP_CLOSED;
             kprint("[TCP] Connection Closed by Remote.\n");
        }
    }
}

/* Connect (Active Open) */
int tcp_connect(uint32_t dst_ip, uint16_t dst_port) {
    g_local_seq = 1000; /* Randomish ISN */
    g_remote_ack = 0;
    g_remote_ip = dst_ip;
    g_remote_port = dst_port;
    g_tcp_state = TCP_SYN_SENT;
    
    kprint("[TCP] Sending SYN...\n");
    tcp_send_packet(dst_ip, dst_port, TCP_FLAG_SYN, NULL, 0);
    
    /* Wait for connection */
    uint64_t start = timer_get_ticks();
    while (g_tcp_state != TCP_ESTABLISHED) {
        if (timer_get_ticks() - start > 300) { /* 3 seconds */
            kprint("[TCP] Connection Timeout.\n");
            g_tcp_state = TCP_CLOSED;
            return -1;
        }
        
        /* Poll Network */
        extern void rtl8139_poll(void);
        rtl8139_poll(); 
        /* Ideally we yield, but polling is needed if IRQs are limited */
        /* __asm__ volatile("hlt"); */
    }
    
    return 0;
}

void tcp_send(uint32_t dst_ip, uint16_t dst_port, const void *data, uint16_t len) {
    if (g_tcp_state != TCP_ESTABLISHED) {
        kprint("[TCP] Cannot send: Not connected.\n");
        return;
    }
    tcp_send_packet(dst_ip, dst_port, TCP_FLAG_PSH | TCP_FLAG_ACK, data, len);
}
