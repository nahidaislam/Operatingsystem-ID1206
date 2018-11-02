# Show that we support Multiboot (with a multiboot header)
# a bootloader can then use it to boot (we use GRUB)
# db - define byte, 8 bits
# dw - define word, 2 bytes
# dd - define double word, 4 byte

section .multiboot_header
header_start:
 dd 0xe85250d6                ; magic number (multiboot 2)
 dd 0                         ; architecture 0 (protected mode i386)
 dd header_end - header_start ; header length
 dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start)) ; checksum, verify that the multiboot header is in fact a multiboot header

 ; insert optional multiboot tags here

 ; required end tag
 dw 0    ; type
 dw 0    ; flags
 dd 8    ; size
header_end:
