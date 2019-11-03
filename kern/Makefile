XDP_SOURCES=xsk_kern.c
XDP_OBJECTS=$(XDP_SOURCES:.c=.o)

all: $(XDP_OBJECTS)

$(XDP_OBJECTS): %.o: %.c
	clang -S -target bpf -I /usr/include/x86_64-linux-gnu -I ../deps/libbpf/src -O2 -emit-llvm -c -g -Wall -o ${@:.o=.ll} $<
	llc -march=bpf -filetype=obj -o $@ ${@:.o=.ll}
	@rm *.ll

clean:
	rm $(XDP_OBJECTS)
