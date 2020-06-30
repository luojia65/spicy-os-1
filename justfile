target := "riscv64imac-unknown-none-elf"
mode := "debug"
kernel_file := "target/" + target + "/" + mode + "/spicy-os"
bin_file := "target/" + target + "/" + mode + "/kernel.bin"

objdump := "rust-objdump --arch-name=riscv64"
objcopy := "rust-objcopy --binary-architecture=riscv64"
size := "rust-size"

build: kernel
    @{{objcopy}} {{kernel_file}} --strip-all -O binary {{bin_file}}

kernel:
    @cargo build --target={{target}}
    
qemu: build
    @qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios default \
            -device loader,file={{bin_file}},addr=0x80200000 \
            -smp threads=4

run: build qemu

asm: build
    @{{objdump}} -D {{kernel_file}} | less

size: build
    @{{size}} -A -x {{kernel_file}}
