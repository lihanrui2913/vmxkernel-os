#include "stdio.h"
#include "stdlib.h"
#include "string.h"

int main()
{
    const char *src = "Hello world!\n";
    char *dst = malloc(13);
    strcpy(dst, src);

    printf(dst);

    return 0;
}
