# Wildcard

`[..]` matches any text within one line.

```console
$ echo 'commit 4f2a91c by alice'
commit [..]
$ echo 'commit 4f2a91c by alice'
commit [..] by alice
$ echo 'commit 4f2a91c by alice'
[..]
```
