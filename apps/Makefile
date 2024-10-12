APPS := test.elf

all: $(APPS)

%.elf: %.c
	x86_64-elf-gcc -o $@.o -c $< -I../newlib/include -O2 -fno-stack-protector
	x86_64-elf-ld -z noexecstack -o $@ $@.o ../newlib/lib/libc.a ../vsc/target/x86_64-unknown-none/debug/libvsc.a
	rm -rf $@.o

clean:
	rm -rf *.elf
