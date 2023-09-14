#include <linux/limits.h>
#include <sys/types.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>

#define CGROUP_PATH "/cgroup2"
#define N_CGROUPS 100
#define ALLOCATION_BYTES 15000000
#define TIMEOUT_SEC 30
#define MEMORY_MAX "10000000"

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

int cg_create(const char *cgroup)
{
	return mkdir(cgroup, 0755);
}

/* Returns 0 on success, or -errno on failure. */
int cg_read(const char *cgroup, const char *control, char *buf, size_t len)
{
	char path[PATH_MAX];
	ssize_t ret;

	snprintf(path, sizeof(path), "%s/%s", cgroup, control);

	ret = read_text(path, buf, len);
	return ret >= 0 ? 0 : ret;
}

int cg_killall(const char *cgroup)
{
	char buf[PATH_MAX];
	char *ptr = buf;

	/* If cgroup.kill exists use it. */
	if (!cg_write(cgroup, "cgroup.kill", "1"))
		return 0;

	if (cg_read(cgroup, "cgroup.procs", buf, sizeof(buf)))
		return -1;

	while (ptr < buf + sizeof(buf)) {
		int pid = strtol(ptr, &ptr, 10);

		if (pid == 0)
			break;
		if (*ptr)
			ptr++;
		else
			break;
		if (kill(pid, SIGKILL))
			return -1;
	}

	return 0;
}

int cg_destroy(const char *cgroup)
{
	int ret;

retry:
	ret = rmdir(cgroup);
	if (ret && errno == EBUSY) {
		cg_killall(cgroup);
		usleep(100);
		goto retry;
	}

	if (ret && errno == ENOENT)
		ret = 0;

	return ret;
}

/* Returns written len on success, or -errno on failure. */
static ssize_t write_text(const char *path, char *buf, ssize_t len)
{
	int fd;

	fd = open(path, O_WRONLY | O_APPEND);
	if (fd < 0)
		return -errno;

	len = write(fd, buf, len);
	close(fd);
	return len < 0 ? -errno : len;
}

/* Returns 0 on success, or -errno on failure. */
int cg_write(const char *cgroup, const char *control, char *buf)
{
	char path[PATH_MAX];
	ssize_t len = strlen(buf), ret;

	snprintf(path, sizeof(path), "%s/%s", cgroup, control);
	ret = write_text(path, buf, len);
	return ret == len ? 0 : ret;
}

int cg_enter(const char *cgroup, int pid)
{
	char pidbuf[64];

	sprintf(pidbuf, "%d", pid);
	return cg_write(cgroup, "cgroup.procs", pidbuf);
}

int stresser(int i, char *cgroup)
{
        char buf[PATH_MAX];
        cg_enter(cgroup, getpid());
        sprintf(buf, "stress --vm 1 --vm-bytes %d -t %d", ALLOCATION_BYTES, TIMEOUT_SEC);
        return system(buf);
}

int spawn_stresser(int i, char *cgroup)
{
        int pid = fork();
        if (pid == 0) {
                exit(stresser(i, cgroup));
        } else {
                return pid;
        }
}

int main (int argc, char **argv)
{
        int pids[N_CGROUPS];
        for (int i = 0; i < N_CGROUPS; i++) {
                char cgroup[PATH_MAX];
                sprintf(cgroup, "%s/foo%d", CGROUP_PATH, i);
                if (cg_create(cgroup)) {
                        printf("Could not create cgroup %s\n", cgroup);
                        return -1;
                }                
                if (cg_write(cgroup, "memory.max", MEMORY_MAX)) {
                        printf("Could not set memory.max %d on %s\n", MEMORY_MAX, cgroup);
                        return -1;
                }

                int child_pid = spawn_stresser(i, cgroup);
                if (child_pid < 0) {
                        printf("Could not spawn child\n.");
                        return -1;
                }
                pids[i] = child_pid;
        }

        for (int i = 0; i < N_CGROUPS; i++) {
                char cgroup[PATH_MAX];
                sprintf(cgroup, "%s/foo%d", CGROUP_PATH, i);

                int ret;
                waitpid(pids[i], &ret, 0);
                pids[i] = ret;

                cg_destroy(cgroup);
        }
        printf("DONE.");
        return 0;
}