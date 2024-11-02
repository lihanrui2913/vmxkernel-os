all: libc
	cargo build --release
	make -C apps

libc:
	make -C relibc install DESTDIR=$(shell pwd)/libc
	rm -rf $(shell pwd)/libc/lib/crt1.o

clean:
	rm -rf $(shell pwd)/libc
	make -C apps clean
