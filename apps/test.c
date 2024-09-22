#include <unistd.h>

int main()
{
    write(STDOUT_FILENO, "Hello C world!\n", 15);
    return 0;
}
