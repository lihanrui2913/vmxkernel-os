#include "stdio.h"
#include "fcntl.h"
#include "unistd.h"

int main()
{
    printf("Hello C world!\n");
    int num1, num2;
    printf("Enter your number 1\n");
    scanf("%d", &num1);
    printf("num1 is %d\n", num1);
    printf("Enter your number 2\n");
    scanf("%d", &num2);
    printf("num2 is %d\n", num2);
    printf("Result is %d\n", num1 + num2);

    int fd = creat("/test.txt", O_RDWR);
    write(fd, "Hello fs world!!!", 17);

    return 0;
}
