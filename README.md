# page-modification-tracking

Experimenting with approaches to track modifications to a process' vm
pages in rust. This repository's Makefile will automatically install
rust using the rustup.rs if rust is not found at ~/.cargo.


To run all of the tests:


```

make test


```


To run the test using tmpfs and default 4k pages:


```

make run-tmpfs-test

```


To run the teest using hugetlbfs and 2M pages:


```

make run-hugetlbfs-test


```


Note that the test assumes you do not have a hugetlbfs mount
configured and will create a 1G mount for you located at
/mnt/beholder-hugetlbfs-test.
