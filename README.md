### VFAT / FAT32

## Limitations
From: https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system#Size_limits
Depending on how the FAT is constructed:
* 1 sector on each copy of FAT for every 128 clusters
* FAT32 range : 65,525 to 268,435,444 clusters : 512 to 2,097,152 sectors per copy of FAT
* FAT32 minimum : 1 sector per cluster × 65,525 clusters = 33,548,800 bytes (32,762.5 KB)
* FAT32 maximum : 8 sectors per cluster × 268,435,444 clusters = 1,099,511,578,624 bytes (≈1,024 GB)
* FAT32 maximum : 16 sectors per cluster × 268,173,557 clusters = 2,196,877,778,944 bytes (≈2,046 GB)


### From Linux
The script below creates a new FS with an MBR and a fat32 partition in it.
```bash 
# From: https://unix.stackexchange.com/a/527217/61495
# Filename of resulting disk image
mkdir /tmp/irisos_fat32/
cd $_
diskimg=fat32.fs
# FS size in megabytes:
fs_size=260   
# Desired size in bytes
size=$((${fs_size}*(1<<20))) 
# align to next MB (https://www.thomas-krenn.com/en/wiki/Partition_Alignment)
alignment=$((1<<20))  
# ceil(size, 1MB):
size=$(( (size + alignment - 1)/alignment * alignment ))  
# mkfs.fat requires size as an (undefined) block-count; seem to be units of 1k
mkfs.fat -C -F32 -n "IRISVOL" "${diskimg}".fat $((size >> 10))
# insert the filesystem to a new file at offset 1MB
dd if=${diskimg}.fat of=${diskimg} conv=sparse obs=512 seek=$((${alignment}/512))
# extend the file by 1MB
truncate -s "+${alignment}" "${diskimg}"
# apply partitioning
parted --align optimal "${diskimg}"\
  mklabel msdos\
  mkpart primary fat32 1MiB 100%\
  set 1 boot on
# Cleanup unneded fat section
rm -f ${diskimg}.fat
```
You can then mount it:
```
dest=/mnt/test
# use fdisk -l fat32.fs to find sector size and sector size
# and then:
sudo mount -o loop,offset=$((2048*512)) fat32.fs /mnt/test/
cd ${dest}
sudo mkdir CaRteLLa
sudo mkdir folder
sudo bash -c 'echo "Hello, Iris OS!"> hello.txt'
sudo touch a-very-long-file-name-entry.txt
```

### Tests resources:
To make the tests work, you'll need to copy the CONTENT of test/resources in your mounted filesystem.
```
├── a-big-file.txt
├── a-very-long-file-name-entry.txt
├── MyFoLdEr
├── folder
│        └── some
│             └── deep
│                 └── nested
│                     └── folder
│                         └── file
└── hello.txt
```




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


### Testing
For vfat integration test, run with `RUST_TEST_TASKS=1`.
All tests runs on same filesystem, and test shall not happen in parallel.

### Useful docs:
* https://www.win.tue.nl/~aeb/linux/fs/fat/fat-1.html