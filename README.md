# VFAT / FAT32
A simple VFAT implementation written in rust, and mostly tested against Linux's vfat driver.

## no_std
This component was first developed with no_std in mind. `std` is still not yet supported but coming soon.

## Run example
to run the example, first create a vfat fs using tests/setup.sh, then run the example file using:
```bash
cargo run --example simple
```

## More info:
* Exfat specification: https://docs.microsoft.com/en-us/windows/win32/fileio/exfat-specification#1-introduction

## Testing
All tests runs on same filesystem, and test shall not happen in parallel (for now!).

To run the setup.sh script, I've added an exception for my user in the sudoers file:
```
fponzi ALL=(ALL) NOPASSWD: /usr/bin/mount,/usr/bin/umount
```
On github actions (CI) it just works, because the user has passwordless sudo.


### Utils:
You can check whether the file contains a valid MBR via `gdisk`:

```bash
$ gdisk -l fat32.fs
```
and you can get info about the filesystem with `fdisk`:
```
$ fdisk -l fat32.fs
```

Some stupid script to flush fs changes:
```
sudo umount /mnt/test
sudo mount -o loop,offset=$((2048*512)) fat32.fs /mnt/test/
ls -l /mnt/test
```

Check the changes:
```shell
sudo dosfsck -w -r -l -v -r /dev/loop13
```


---

### TODO
* Have a better update entry which allows to support renaming.
* Find a way to get a "timestamp" function in the fs.
* Add error type
* Test: What happens if there are no free clusters (memory is full)?

## Long todo:
* Support longer files (rn it just creates 1 lfn per entry).
* Write to backup FAT as well.
* Free cluster summary update when allocating clusters.

### Future improvements.
* Currently, the device mutex is shared behind an ARC reference. Maybe, also having the whole FS behind arc would save quite some space when
  returning files and directories. Because they get a copy of the Vfat struct.
* Get rid of alloc dependency? -> only used for String support rn.

### FAQ
* What happens if I have a "File" handle and meanwhile someone deletes this file and
  I try to read from a deleted file?
  This case should be taken care of by the application using this library.



### Useful docs:
* https://www.win.tue.nl/~aeb/linux/fs/fat/fat-1.html

---

To mount with 777 permission:

```
sudo mount -o loop,offset=$((2048*512)),uid=1000,gid=1000,dmask=0000,fmask=0001 fat32.fs /mnt/test/
```