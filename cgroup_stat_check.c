#include <linux/limits.h>
#include <sys/types.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>

#define STAT_FILE_PATH "/cgroup2/memory.stat"
#define BUF_LEN 4096
#define ITERATIONS 50000

/* Returns read len on success, or -errno on failure. */
static ssize_t read_text(const char *path, char *buf, size_t max_len)
{
	ssize_t len;
	int fd;

	fd = open(path, O_RDONLY);
	if (fd < 0)
		return -errno;

	len = read(fd, buf, max_len - 1);

	if (len >= 0)
		buf[len] = 0;

	close(fd);
	return len < 0 ? -errno : len;
}


int main (int argc, char **argv)
{
        for (int i=0; i<ITERATIONS;i++) {
                char buf[BUF_LEN];
                if (read_text(STAT_FILE_PATH, buf, BUF_LEN) < 0) {
                        printf("Could not read stat file %s\n", STAT_FILE_PATH);
                        return -1;
                }
        }
        return 0;
}