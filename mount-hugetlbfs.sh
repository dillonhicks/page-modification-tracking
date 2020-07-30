#!/usr/bin/env bash
set -x
set -e

DEFAULT_HUGE_MNT_SIZE_GB=2
DEFAULT_HUGE_MNT_PATH=/mnt/huge


main() {
    let huge_mnt_size_gb=${DEFAULT_HUGE_MNT_SIZE_GB}
    let huge_mnt_pagecount=$(((huge_mnt_size_gb * 1024)/2))
    huge_mnt_path=${DEFAULT_HUGE_MNT_PATH}
    sudo -E mkdir -pv ${huge_mnt_path}
    sudo -n -E chown -Rv $(id -u $(whoami)):$(id -g $(whoami)) ${huge_mnt_path}
    sudo -n -E /usr/bin/env bash -c "set -x; (echo ${huge_mnt_pagecount} > /proc/sys/vm/nr_hugepages)"
    (sudo -n -E umount -v ${huge_mnt_path} || true)
    sudo -n -E mount \
         -v \
         -t hugetlbfs \
         -o defaults,uid=$(id -u $(whoami)),gid=$(id -g $(whoami)),size=${huge_mnt_size_gb}G,pagesize=2M \
         none \
         ${huge_mnt_path}
}


main $@
