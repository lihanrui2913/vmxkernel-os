#include "unistd.h"
#include "sys/wait.h"

int main()
{
    int pid = execve("/root/shell.elf", (char *const *)"ls", (char **)0);
    waitpid(pid, (int *)0, 0);

    return 0;
}
