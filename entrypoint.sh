#!/bin/bash
set -e

# SSH host keys
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A 2>/dev/null
fi

# Root password (default: gitpage)
ROOT_PASS="${SSH_ROOT_PASSWORD:-gitpage}"
echo "root:$ROOT_PASS" | chpasswd -c SHA512 2>/dev/null

# Create per-user SSH accounts from SSH_USERS env var
# Format: "user1:pass1,user2:pass2"  e.g. "alice:alice123,bob:bob123"
IFS=',' read -ra USER_LIST <<< "${SSH_USERS:-}"
for entry in "${USER_LIST[@]}"; do
    IFS=':' read -r uname upass <<< "$entry"
    if [ -n "$uname" ] && [ -n "$upass" ]; then
        id "$uname" 2>/dev/null || useradd -m -d "/home/$uname" -s /bin/bash "$uname"
        echo "$uname:$upass" | chpasswd -c SHA512 2>/dev/null

        # Write sync script: clones missing repos on login
        cat > "/home/$uname/.gitpage-sync" << 'SYNCSCRIPT'
#!/bin/bash
bare_dir="/app/data/repos/$(whoami)"
if [ ! -d "$bare_dir" ]; then
    return 0 2>/dev/null || exit 0
fi
for bare_repo in "$bare_dir"/*.git/; do
    [ -d "$bare_repo" ] || continue
    repo_name=$(basename "$bare_repo" .git)
    clone_dest="$HOME/$repo_name"
    if [ -d "$bare_repo/objects" ] && ls "$bare_repo/objects/"* 2>/dev/null | grep -q .; then
        if [ ! -d "$clone_dest/.git" ]; then
            echo "  sync: cloning $repo_name..."
            git clone -q "$bare_repo" "$clone_dest" 2>/dev/null
            chown -R "$(whoami):$(whoami)" "$clone_dest" 2>/dev/null || true
        fi
    fi
done
SYNCSCRIPT
        chown "$uname:$uname" "/home/$uname/.gitpage-sync"
        chmod +x "/home/$uname/.gitpage-sync"

        # Source sync script in .bashrc
        if ! grep -q "gitpage-sync" "/home/$uname/.bashrc" 2>/dev/null; then
            echo "" >> "/home/$uname/.bashrc"
            echo "# Sync gitpage repos on login" >> "/home/$uname/.bashrc"
            echo "source ~/.gitpage-sync" >> "/home/$uname/.bashrc"
        fi

        echo "  Created SSH user: $uname"
    fi
done

# Start sshd
mkdir -p /run/sshd
/usr/sbin/sshd -D &
sleep 1

echo ""
echo "=== SSH users ==="
echo "  root / $ROOT_PASS"
for entry in "${USER_LIST[@]}"; do
    IFS=':' read -r uname upass <<< "$entry"
    [ -n "$uname" ] && [ -n "$upass" ] && echo "  $uname / $upass"
done
echo ""
echo "=== ssh root@localhost -p \$SSH_PORT ==="
echo ""

exec gitpage "$@"
