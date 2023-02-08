obj-m := kmallocer.o
all:
	make -C ../../linux/ M=$(PWD) modules
	cp kmallocer.ko ../../shared/

clean:
	make -C ../../linux M=$(PWD) clean