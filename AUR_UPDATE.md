### AUR manual update

```bash
makepkg -od

makepkg --printsrcinfo > .SRCINFO

git add PKGBUILD .SRCINFO
git commit -m "AUR Update"
git push origin master

```
