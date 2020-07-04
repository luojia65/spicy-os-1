target := "riscv64imac-unknown-none-elf"
mode := "debug"
kernel_file := "target/" + target + "/" + mode + "/spicy-os"
bin_file := "target/" + target + "/" + mode + "/kernel.bin"

# USER_DIR	:= ../user
# USER_BUILD	:= $(USER_DIR)/build
# IMG_FILE	:= $(USER_BUILD)/disk.img
img_file := "disk/disk.img"

# objdump := "rust-objdump --arch-name=riscv64"
objdump := "riscv64-unknown-elf-objdump"
objcopy := "rust-objcopy --binary-architecture=riscv64"
gdb := "riscv64-unknown-elf-gdb"
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
    		-drive file={{img_file}},format=qcow2,id=sfs \
    		-device virtio-blk-device,drive=sfs \
            -smp threads=1

run: build qemu

asm: build
    @{{objdump}} -D {{kernel_file}} | less

size: build
    @{{size}} -A -x {{kernel_file}}

debug: build
    @qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios default \
            -device loader,file={{bin_file}},addr=0x80200000 \
            -smp threads=1 \
            -gdb tcp::11111 -S
gdb: 
    @gdb --eval-command="file {{kernel_file}}" --eval-command="target remote localhost:11111"

    