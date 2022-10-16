FROM nixos/nix
RUN nix-shell -p busybox --command "adduser test -D -h /github/home"
USER test