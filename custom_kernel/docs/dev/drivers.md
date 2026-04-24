# Device Driver Model

Ainux uses a monolithic driver model where drivers are compiled directly into the kernel binary for maximum performance and minimum latency.

## 1. Video Drivers
*   **VBE / BGA**: Support for high-resolution linear framebuffers.
*   **NuxV Engine**: A custom graphics layer that provides hardware-accelerated TUI rendering and font management.

## 2. Network Drivers
*   **RTL8139**: Support for the Realtek 8139 PCI ethernet card.
*   **Atheros WiFi**: (Experimental) Support for wireless connectivity.

## 3. Storage Drivers
*   **ATA / PATA**: Legacy disk support for IDE drives.
*   **AHCI**: (Planned) Modern SATA support.

## 4. Input Drivers
*   **PS/2 Keyboard**: Standard interrupt-driven keyboard support.
*   **USB Mouse**: Support for pointing devices via the EHCI controller.

## 5. Driver API
All drivers are required to implement a standardized `Driver` trait (planned) that includes `init`, `start`, and `stop` methods.
