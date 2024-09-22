APPS := test.elf

all: $(APPS)

%.elf: %.c
	gcc -o $@.o -c $< -I../newlib/include -fno-stack-protector
	ld -z noexecstack -o $@ $@.o ../newlib/lib/libc.a ../vsc/target/x86_64-unknown-none/debug/libvsc.a
	rm -rf $@.o

clean:
	rm -rf *.elf
