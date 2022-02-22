### VFAT / FAT32

## Limitations
From: https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system#Size_limits
Depending on how the FAT is constructed:
* 1 sector on each copy of FAT for every 128 clusters
* FAT32 range : 65,525 to 268,435,444 clusters : 512 to 2,097,152 sectors per copy of FAT
* FAT32 minimum : 1 sector per cluster × 65,525 clusters = 33,548,800 bytes (32,762.5 KB)
* FAT32 maximum : 8 sectors per cluster × 268,435,444 clusters = 1,099,511,578,624 bytes (≈1,024 GB)
* FAT32 maximum : 16 sectors per cluster × 268,173,557 clusters = 2,196,877,778,944 bytes (≈2,046 GB)


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
* Currently the mutex is shared behind an ARC reference. Maybe, having the whole FS behind arc would save quite some space when
  returning files and directories.
* Get rid of alloc dependency?

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