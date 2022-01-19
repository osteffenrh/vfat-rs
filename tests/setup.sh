#!/usr/bin/env bash
set -e
# FS size in megabytes:
fs_size=260
# Desired size in bytes
size=$((${fs_size}*(1<<20)))
# align to next MB (https://www.thomas-krenn.com/en/wiki/Partition_Alignment)
alignment=$((1<<20))
# ceil(size, 1MB):
size=$(( (size + alignment - 1)/alignment * alignment ))

temp_dir=/tmp/irisos_fat32/
diskimg=fat32.fs

echo "setup.sh: going to create an fs in ${temp_dir}${diskimg}";

# From: https://unix.stackexchange.com/a/527217/61495
# Filename of resulting disk image
mkdir -p $temp_dir
cd $temp_dir

# mkfs.fat requires size as an (undefined) block-count; seem to be units of 1k
mkfs.fat -C -F32 -n "IRISVOL" "${diskimg}".fat $((size >> 10))

# insert the filesystem to a new file at offset 1MB
dd if=${diskimg}.fat of=${diskimg} conv=sparse obs=512 seek=$((${alignment}/512))

# extend the file by 1MB
truncate -s "+${alignment}" "${diskimg}"

# apply partitioning
parted -s --align optimal "${diskimg}"\
  mklabel msdos\
  mkpart primary fat32 1MiB 100%\
  set 1 boot on

# Cleanup unneeded fat section
rm -fv ${temp_dir}/fat32.fs.fat
