#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/vmalloc.h>
#include <linux/delay.h>

#define BUFSIZE 512
#define MAX_ALLOCATION_NODES 20000


static int iterations = 5;
static u64 max_iteration_time_ns = 1000 * 1000 * 15;
static unsigned long int max_allocation = 1000 * 1000 * 100;
static unsigned long int hold_time_ms = 500;
static size_t min_allocation_size = 1 << 13;
static size_t max_allocation_size = 1 << 16;

static const char *dir_name = "kmallocer";
static struct proc_dir_entry *dir_entry;

static const char *do_entry_name = "do";
static struct proc_dir_entry *do_entry;

static const char *conf_entry_name = "conf";
static struct proc_dir_entry *conf_entry;

struct allocation_node {
    void *allocation;
    struct allocation_node *next;
};

static unsigned long int allocation_loop(void) 
{
    unsigned long int tot_allocated, allocation_size, i = 0;
    struct allocation_node *root_node, *tail_node, *tmp_node;
    int successful_allocation_streak = 0;
    u64 start_time, elapsed;

    tot_allocated = 0;
    allocation_size = min_allocation_size;
    root_node = kzalloc(sizeof(struct allocation_node), GFP_KERNEL);
    if (!root_node) {
        return 0;
    }
    tail_node = root_node;
    start_time = ktime_get_ns();    
    for(;;) {        
        void* allocated = kmalloc(allocation_size, __GFP_HIGH);
        if (!allocated) {
            allocation_size = allocation_size >> 1;
            allocation_size = allocation_size < min_allocation_size ? min_allocation_size : allocation_size;
        } else {
            tmp_node = allocated;
            tmp_node->allocation = allocated;
            tmp_node->next = NULL;

            tail_node->next = tmp_node;
            tail_node = tmp_node;

            tot_allocated += allocation_size;
            if (++successful_allocation_streak >= 3) {
                successful_allocation_streak = 0;
                allocation_size = allocation_size << 1;                
                allocation_size = allocation_size > max_allocation_size ? max_allocation_size : allocation_size;
            }
        }
        i++;
        if (tot_allocated > max_allocation) {
            break;
        }
        elapsed = ktime_get_ns() - start_time;
        if (elapsed > max_iteration_time_ns) {
            break;
        }
    }
    mdelay(hold_time_ms);
    
    tmp_node = root_node;
    while (tmp_node) {
        root_node = tmp_node->next;
        kfree(tmp_node->allocation);
        tmp_node = root_node;
    }

    return tot_allocated;
}

static void fmt_bytes(unsigned long int bytes, char* buf) {
    char suffix;
    unsigned long int divisor;
    if(bytes < 1000) {
        suffix = ' ';
        divisor = 1;
    } else if (bytes < 1000000) {
        suffix = 'K';
        divisor = 1000;
    } else {
        suffix = 'M';
        divisor = 1000000;
    }
    sprintf(buf, "%5ld%c", bytes / divisor, suffix);
}

static ssize_t perform_burst_and_print(struct file *file, char __user *ubuf, size_t count, loff_t *ppos)
{

    unsigned long int allocated,tot_allocated = 0, peak = 0, min = ~0 ;
    char buf[BUFSIZE];
    char fmt_buf[64];
    int len = 0;

    printk("performing burst file: %p, ubuf: %p count: %ld, ppos %lld", file, ubuf, count, *ppos);

    if (*ppos > 0 || count < BUFSIZE)
        return 0;

    for (int i=0;i<iterations;i++) {
        allocated = allocation_loop();
        peak = allocated > peak ? allocated : peak;
        min = allocated < min ? allocated : min;
        tot_allocated += allocated;
        fmt_bytes(allocated, fmt_buf);
        len += sprintf(buf + len, "%s\n", fmt_buf);
    }

    fmt_bytes(peak, fmt_buf);
    len += sprintf(buf + len, "peak: %s ", fmt_buf);

    fmt_bytes(min, fmt_buf);
    len += sprintf(buf + len, "min: %s ", fmt_buf);

    fmt_bytes(tot_allocated / iterations, fmt_buf);
    len += sprintf(buf + len, "avg: %s\n", fmt_buf);

    if (copy_to_user(ubuf, buf, len))
        return -EFAULT;

    *ppos = len;
    return len;
}

static ssize_t conf_read(struct file *file, char __user *ubuf, size_t count, loff_t *ppos)
{
    int len;
    char buf[BUFSIZE];
    printk("[ASD] conf read %p %ld %p", ubuf, count, ppos);
     if (*ppos > 0)
         return 0;
    if (count < BUFSIZE)
        return -EFAULT;
    
    len = sprintf(buf, "%d %ld %ld %ld %lld %ld", iterations, max_allocation, min_allocation_size, max_allocation_size, max_iteration_time_ns, hold_time_ms);

    if (copy_to_user(ubuf, buf, len))
        return -EFAULT;

    *ppos = len;
    return len;
}

static ssize_t conf_write(struct file *file, const char __user *ubuf, size_t count, loff_t *ppos)
{
    int in_iterations, scanned_values;
    u64 in_max_iteration_time_ns;
    unsigned long int in_max_allocation, in_hold_time_ms;
    size_t in_min_allocation_size, in_max_allocation_size;

    char buf[BUFSIZE];
    if (*ppos > 0 || count > BUFSIZE)
        return -EFAULT;

    if (copy_from_user(buf, ubuf, count))
        return -EFAULT;

    scanned_values = sscanf(buf, "%d %ld %ld %ld %lld %ld", &in_iterations, &in_max_allocation, &in_min_allocation_size, &in_max_allocation_size, &in_max_iteration_time_ns, &in_hold_time_ms);
    if (scanned_values != 6) 
        return -EFAULT;
    
      iterations = in_iterations;
      max_iteration_time_ns = in_max_iteration_time_ns;
      max_allocation = in_max_allocation;
      hold_time_ms = in_hold_time_ms;
      min_allocation_size = in_min_allocation_size;
      max_allocation_size = in_max_allocation_size;

	return count;
}

static struct proc_ops do_ops =
{
        .proc_read = perform_burst_and_print,
};

static struct proc_ops conf_ops =
{
        .proc_read = conf_read,
        .proc_write = conf_write,
};

int init_module(void)
{
    printk("kmallocer mod init.\n");
    dir_entry = proc_mkdir(dir_name, NULL);
    do_entry = proc_create(do_entry_name, 0660, dir_entry, &do_ops);
    conf_entry = proc_create(conf_entry_name, 0660, dir_entry, &conf_ops);
    if (!do_entry || !dir_entry || !conf_entry)
    {
        return -ENOMEM;
    }
    printk("kmallocer mod init DONE.\n");
    return 0;
}

void cleanup_module(void)
{
    remove_proc_entry(do_entry_name, dir_entry);
    remove_proc_entry(conf_entry_name, dir_entry);
    remove_proc_entry(dir_name, NULL);
    printk("kmallocer mod unloaded.\n");
}

MODULE_LICENSE("GPL v2");