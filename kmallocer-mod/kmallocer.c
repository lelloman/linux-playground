#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/vmalloc.h>
#include <linux/delay.h>

#define BUFSIZE 1024
#define MAX_ALLOCATION_NODES 20000

static struct proc_dir_entry *ent;

static int iterations = 5;
static long unsigned int max_allocation = 1000 * 1000 * 100;
static size_t min_allocation_size = 1 << 13;
static size_t max_allocation_size = 1 << 16;
static u64 max_iteration_time_ns = 1000 * 1000 * 15;
static unsigned long int hold_time_ms = 500;

static const char *entry_name = "kmallocer";

static char buf[BUFSIZE];

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
        void* allocated = kmalloc(allocation_size, __GFP_ATOMIC|__GFP_HIGH);
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
    char fmt_buf[512];
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

static struct proc_ops myops =
{
        .proc_read = perform_burst_and_print,
};

int init_module(void)
{
    printk(KERN_INFO "kmallocer mod init.\n");
    ent = proc_create(entry_name, 0660, NULL, &myops);
    if (!ent)
    {
        return -ENOMEM;
    }
    printk(KERN_INFO "kmallocer mod init DONE.\n");
    return 0;
}

void cleanup_module(void)
{
    remove_proc_entry(entry_name, NULL);
    printk(KERN_INFO "kmallocer mod unloaded.\n");
}

MODULE_LICENSE("GPL v2");