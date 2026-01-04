void c_hardware_init() {
    // Write to standard serial port 0x3F8
    // Since we don't have standard library, we use inline assembly or rely on
    // proper linking if we had a library. For now, just a dummy function 
    // that might write to a memory location or just return.
    
    // To prove it runs, let's write to a known address if we can, 
    // or just leave it empty but confirm it links.
    // Better: use inline asm to write 'C' to serial.
    
    unsigned short port = 0x3F8;
    unsigned char c = 'C';
    __asm__ volatile ("outb %0, %1" : : "a"(c), "Nd"(port));
}
