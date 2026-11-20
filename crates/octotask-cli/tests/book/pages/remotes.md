# Remotes

The fixture remotes, at their GitHub urls.

```console
$ git ls-remote git@github.com:acme/shop-frontend.git
2736d78cdd9866ea409256e5cb6248dcda6c9b7f	HEAD
1695c7408d7098621edc8abd2aebf9ad8fb6d87d	refs/heads/feat/login
2736d78cdd9866ea409256e5cb6248dcda6c9b7f	refs/heads/main
1ec5189b3d508357766f6d8b51dbf14a45ee8d3b	refs/heads/staging
$ git ls-remote git@github.com:acme/shop-backend.git
9eefcbb766b6002422faa5324814ca8c9fb5d69d	HEAD
ec75932d921199496e320127d83287369c97d584	refs/heads/feat/login
9eefcbb766b6002422faa5324814ca8c9fb5d69d	refs/heads/main
de0859773f10821a8ae2ee138dc2bd158b53fd46	refs/heads/staging
$ git ls-remote git@github.com:acme/shop-agent.git
f5e0f0d946140f58fb6b3cf262bd855923f5b22f	HEAD
1967eeae28b423e236317c26b4a4d49afdee2be7	refs/heads/fix/prompt
f5e0f0d946140f58fb6b3cf262bd855923f5b22f	refs/heads/main
a1ea3885b58c3f473fdc1db7796cb62e45c2b58d	refs/heads/staging
$ git clone --quiet git@github.com:acme/shop-backend.git
$ cd shop-backend
$ git remote -v
origin	git@github.com:acme/shop-backend.git (fetch)
origin	git@github.com:acme/shop-backend.git (push)
$ git log --format='%h %an %cr %s' origin/feat/login
ec75932 Alice 3 hours ago Add login endpoint
de08597 Alice 5 hours ago Fix order totals
9eefcbb Alice 10 days ago Add orders API
083ac21 Alice 3 weeks ago Initial commit
```
