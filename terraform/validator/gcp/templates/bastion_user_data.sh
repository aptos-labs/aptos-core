#!/bin/sh

cat > /usr/local/bin/proxy <<"EOF"
#!/bin/bash
exec 3<>/dev/tcp/$1/$2
cat <&3 & cat >&3
EOF

chmod +x /usr/local/bin/proxy
