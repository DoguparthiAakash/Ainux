#include "dns.h"
#include "udp.h"
#include "net.h"
#include "../libc/string.h"
#include "../libc/stdio.h"
#include "../log.h"
#include "../ex/mm/heap.h"
#include "net.h" /* For ntohs/htons */

#define DNS_SERVER_IP 0x0A000203 /* 10.0.2.3 (QEMU DNS) */
#define DNS_SRC_PORT  12345

/* DNS Header Struct */
typedef struct {
    uint16_t id;
    uint16_t flags;
    uint16_t q_count;
    uint16_t ans_count;
    uint16_t auth_count;
    uint16_t add_count;
} __attribute__((packed)) dns_header_t;

/* Helper to convert www.google.com -> 3www6google3com0 */
static void dns_format_hostname(char *qname, char *hostname) {
    int lock = 0;
    strcat(hostname, "."); /* Append root dot safely assumes buffer space, danger? */
    
    char *host = hostname;
    int len = strlen(host);
    
    int i;
    for(i = 0; i < len; i++) {
        if(host[i] == '.') {
            *qname++ = i - lock;
            for(; lock < i; lock++) {
                *qname++ = host[lock];
            }
            lock++; 
        }
    }
    *qname++ = 0;
}

static void dns_callback(uint8_t *data, uint32_t len, uint32_t src_ip, uint16_t src_port) {
    (void)len; (void)src_ip; /* Unused */
    if (src_port != 53) return;
    
    dns_header_t *dns = (dns_header_t*)data;
    /* printf("[DNS] Response Received. ID: %x, Answers: %d\n", ntohs(dns->id), ntohs(dns->ans_count)); */
    
    if (ntohs(dns->ans_count) > 0) {
         /* Skip Header */
        uint8_t *ptr = data + sizeof(dns_header_t);
        
        /* Skip Query Section */
        while (*ptr != 0) ptr++;
        ptr++; /* 0 byte */
        ptr += 4; /* Type + Class */
        
        /* Now at Answer */
        /* Name is compressed (0xC0XX) usually */
        if ((*ptr & 0xC0) == 0xC0) {
            ptr += 2;
        } else {
             while (*ptr != 0) ptr++;
             ptr++;
        }
        
        ptr += 2; /* Type */
        ptr += 2; /* Class */
        ptr += 4; /* TTL */
        
        uint16_t data_len = (ptr[0] << 8) | ptr[1];
        ptr += 2;
        
        if (data_len == 4) {
             char tmp[64];
             sprintf(tmp, "DNS Resolution: %d.%d.%d.%d\n", ptr[0], ptr[1], ptr[2], ptr[3]);
             kprint(tmp);
        }
    }
}

int dns_resolve(char *hostname) {
    /* 1. Format Packet */
    uint8_t buf[512];
    dns_header_t *dns = (dns_header_t*)buf;
    
    dns->id = htons(0x1337);
    dns->flags = htons(0x0100); /* Standard Query, Recursion Desired */
    dns->q_count = htons(1);
    dns->ans_count = 0;
    dns->auth_count = 0;
    dns->add_count = 0;
    
    /* 2. Format Query */
    uint8_t *qname = buf + sizeof(dns_header_t);
    
    /* Convert hostname (e.g., google.com) to QName */
    char host_copy[256];
    strcpy(host_copy, hostname);
    dns_format_hostname((char*)qname, host_copy);
    
    uint32_t qname_len = strlen((char*)qname) + 1;
    
    uint8_t *qinfo = qname + qname_len;
    *((uint16_t*)qinfo) = htons(1); /* Type A */
    qinfo += 2;
    *((uint16_t*)qinfo) = htons(1); /* Class IN */
    
    uint32_t packet_len = sizeof(dns_header_t) + qname_len + 4;
    
    /* 3. Send UDP */
    /* printf("[DNS] Sending Query for %s to 10.0.2.3\n", hostname); */
    udp_send(DNS_SERVER_IP, DNS_SRC_PORT, 53, buf, packet_len);
    
    return 1;
}

void dns_init(void) {
    udp_bind(DNS_SRC_PORT, dns_callback);
}
