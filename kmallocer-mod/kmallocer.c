#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/vmalloc.h>
#include <linux/delay.h>

#define BUFSIZE 1024

static struct proc_dir_entry *ent;

static int iterations = 5;
static long unsigned int max_allocation = 50000000;
static size_t min_allocation_size = 1 << 14;
static size_t max_allocation_size = 1 << 20;
static u64 max_iteration_time_ns = 1000 * 1000 * 20;
static unsigned long int hold_time_ms = 250;

static const char *entry_name = "kmallocer";

static char buf[BUFSIZE];

static ssize_t dummy_write(struct file *file, const char __user *ubuf, size_t count, loff_t *ppos)
{
    printk("Nothing to write here.");
    return 0;
}

struct allocation_node {
    struct allocation_node* next;
    void* ptr;
};

static unsigned long int allocation_loop(void) 
{
    struct allocation_node* new_node, *prev_root, *root = NULL;
    unsigned long int tot_allocated, allocation_size, i = 0;
    int successful_allocation_streak = 0;
    u64 start_time, elapsed;

    tot_allocated = 0;
    allocation_size = min_allocation_size;
    start_time = ktime_get_ns();    
    for(;;) {        
        struct capukkion* allocated = kzalloc(allocation_size, __GFP_ATOMIC|__GFP_HIGH);
        if (!allocated) {
            size_t next_allocation_size = allocation_size >> 1;
            if (next_allocation_size < min_allocation_size) {
                next_allocation_size = min_allocation_size;
//                break;
            }
            //printk("kmalloc %d failed", allocation_size);
            allocation_size = next_allocation_size;
        } else {
            //printk("kmalloc %d OK", allocation_size);
            memset(allocated, 19, allocation_size);
            new_node = vmalloc(sizeof(struct allocation_node));
            tot_allocated += allocation_size;
            if (!new_node) {
                printk("Failed to vmalloc");
                break;
            }
            prev_root = root;
            root = new_node;
            new_node->next = prev_root;
            new_node->ptr = allocated;
            
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
    
    while(root) {
        struct allocation_node* popped = root;
        root = popped->next;
        kfree(popped->ptr);
        vfree(popped);
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

    unsigned long int allocated,tot_allocated = 0, peak = 0;
    char fmt_buf[512];
    int len = 0;

    printk("myread file: %p, ubuf: %p count: %ld, ppos %p", file, ubuf, count, ppos);

    if (*ppos > 0 || count < BUFSIZE)
        return 0;

    for (int i=0;i<iterations;i++) {
        allocated = allocation_loop();
        peak = allocated > peak ? allocated : peak;
        tot_allocated += allocated;
        fmt_bytes(allocated, fmt_buf);
        len += sprintf(buf + len, "%s\n", fmt_buf);
    }

    fmt_bytes(peak, fmt_buf);
    len += sprintf(buf + len, "peak: %s ", fmt_buf);

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
        .proc_write = dummy_write,
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