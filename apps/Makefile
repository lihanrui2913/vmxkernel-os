APPS := test.elf

all: $(APPS)

%.elf: %.c
	gcc -o $@.o -c $< -I../libc/include -O0 -fno-stack-protector -fno-builtin -nostdinc -nostdlib
	ld ../libc/lib/crt*.o ../libc/lib/lib*.a $@.o -o $@
	rm -rf $@.o

clean:
	rm -rf *.elf
