#!/bin/bash

set -e

VERSION="0.2.0"
PACKAGE_NAME="burrow-db"
ARCH="amd64"
BUILD_DIR="/tmp/burrow-db-build"
PACKAGE_DIR="$BUILD_DIR/$PACKAGE_NAME-$VERSION"

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║                    Building BurrowDB .deb Package                          ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Clean and create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$PACKAGE_DIR"

echo "📦 [1/5] Building binaries..."
cd /home/rdj/Documents/burrowDB/burrow_db/burrow_client
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" || true

echo "📦 [2/5] Creating package structure..."
mkdir -p "$PACKAGE_DIR/DEBIAN"
mkdir -p "$PACKAGE_DIR/usr/bin"
mkdir -p "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME"
mkdir -p "$PACKAGE_DIR/var/lib/$PACKAGE_NAME"
mkdir -p "$PACKAGE_DIR/etc/$PACKAGE_NAME"

echo "📦 [3/5] Installing files..."
install -m 755 /home/rdj/Documents/burrowDB/burrow_db/burrow_client/target/release/burrow-cli "$PACKAGE_DIR/usr/bin/"
install -m 644 /home/rdj/Documents/burrowDB/burrow_db/README.md "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/"
install -m 644 /home/rdj/Documents/burrowDB/burrow_db/LAUNCH.md "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/"
install -m 644 /home/rdj/Documents/burrowDB/burrow_db/CODE_SUMMARY.md "$PACKAGE_DIR/usr/share/doc/$PACKAGE_NAME/"

echo "📦 [4/5] Creating control files..."
cat > "$PACKAGE_DIR/DEBIAN/control" << 'CONTROL'
Package: burrow-db
Version: 0.2.0
Architecture: amd64
Maintainer: RDJ2000
Homepage: https://github.com/RDJ2000/burrowdb
Depends: libc6 (>= 2.17)
Section: database
Priority: optional
Description: High-Performance Document Database with Hot-Cold Tiering
 BurrowDB is a lightweight, high-performance document database designed for
 real-time applications. It uses FlatBuffers for efficient serialization and
 implements intelligent hot-cold tiering for optimal performance.
 .
 Features:
  - Pure FlatBuffer serialization (zero-copy)
  - Hot-cold tiering (RAM + Disk)
  - LRU eviction policy
  - Block-based document storage
  - JSON document support
  - Interactive CLI with verbose mode
  - Real-time statistics
CONTROL

cat > "$PACKAGE_DIR/DEBIAN/postinst" << 'POSTINST'
#!/bin/bash
set -e

case "$1" in
    configure)
        mkdir -p /var/lib/burrow-db /etc/burrow-db /var/log/burrow-db
        chmod 755 /var/lib/burrow-db /etc/burrow-db /var/log/burrow-db
        echo "✅ BurrowDB v0.2.0 installed successfully!"
        echo ""
        echo "Quick Start:"
        echo "  burrow-cli help              - Show help"
        echo "  burrow-cli put key '{...}'   - Store a document"
        echo "  burrow-cli get key           - Retrieve a document"
        echo "  burrow-cli list              - List all documents"
        echo "  burrow-cli stats             - Show statistics"
        echo ""
        ;;
esac
exit 0
POSTINST

chmod 755 "$PACKAGE_DIR/DEBIAN/postinst"

echo "📦 [5/5] Building .deb package..."
cd "$BUILD_DIR"
dpkg-deb --build "$PACKAGE_NAME-$VERSION" 2>&1 | grep -v "^$" || true

# Try both naming conventions
DEB_FILE="$BUILD_DIR/${PACKAGE_NAME}-${VERSION}.deb"
if [ ! -f "$DEB_FILE" ]; then
    DEB_FILE="$BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
fi

if [ -f "$DEB_FILE" ]; then
    cp "$DEB_FILE" /home/rdj/Documents/burrowDB/burrow-db_0.2.0_amd64.deb
    echo ""
    echo "╔════════════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                            ║"
    echo "║  ✅ .deb Package Built Successfully!                                      ║"
    echo "║                                                                            ║"
    echo "║  Package: /home/rdj/Documents/burrowDB/burrow-db_0.2.0_amd64.deb"
    echo "║  Size: $(du -h "$DEB_FILE" | cut -f1)"
    echo "║                                                                            ║"
    echo "║  Installation:                                                            ║"
    echo "║    sudo dpkg -i burrow-db_0.2.0_amd64.deb                                ║"
    echo "║                                                                            ║"
    echo "║  Verification:                                                            ║"
    echo "║    dpkg -l | grep burrow-db                                              ║"
    echo "║    burrow-cli help                                                        ║"
    echo "║                                                                            ║"
    echo "╚════════════════════════════════════════════════════════════════════════════╝"
else
    echo "❌ Failed to build .deb package"
    exit 1
fi
