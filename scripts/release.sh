#!/bin/bash

# RAZ Release Script
# Bumps version, publishes to crates.io, and publishes VS Code extension

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "raz-core" ]]; then
    print_error "This script must be run from the RAZ root directory"
    exit 1
fi

# Get version argument
if [[ $# -eq 0 ]]; then
    print_error "Usage: $0 <version> [--pre-release]"
    print_error "Example: $0 0.1.4"
    print_error "Example: $0 0.1.4-beta.1 --pre-release"
    exit 1
fi

NEW_VERSION="$1"
PRE_RELEASE="${2:-}"

# Validate version format
if [[ $PRE_RELEASE == "--pre-release" ]]; then
    # Allow pre-release versions like 0.1.4-beta.1, 0.1.4-alpha.1, etc.
    if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+(\.[0-9]+)?)?$ ]]; then
        print_error "Invalid pre-release version format. Use semantic versioning (e.g., 0.1.4-beta.1)"
        exit 1
    fi
    IS_PRE_RELEASE=true
else
    # Standard release version
    if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        print_error "Invalid version format. Use semantic versioning (e.g., 0.1.4)"
        exit 1
    fi
    IS_PRE_RELEASE=false
fi

if [[ $IS_PRE_RELEASE == true ]]; then
    print_step "Starting PRE-RELEASE process for version $NEW_VERSION"
else
    print_step "Starting release process for version $NEW_VERSION"
fi

# Check if git is clean
if ! git diff-index --quiet HEAD --; then
    print_error "Git working directory is not clean. Please commit or stash changes."
    exit 1
fi

# Backup current state
print_step "Creating backup of current state"
git stash push -m "pre-release-backup-$(date +%Y%m%d-%H%M%S)" || true

# Array of crates to update (order matters for dependencies)
CRATES=(
    "raz-common"
    "raz-config"
    "raz-validation"
    "raz-override"
    "raz-core"
    "raz-adapters/cli"
)

print_step "Updating versions in all Cargo.toml files"

# Update workspace dependencies version
print_step "Updating workspace dependencies"
sed -i.bak "s/raz-validation = { path = \"raz-validation\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-validation = { path = \"raz-validation\", version = \"$NEW_VERSION\" }/g" Cargo.toml

# Update each crate's version
for crate in "${CRATES[@]}"; do
    print_step "Updating $crate version to $NEW_VERSION"
    
    if [[ -f "$crate/Cargo.toml" ]]; then
        # Update the version field
        sed -i.bak "s/^version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\"/version = \"$NEW_VERSION\"/" "$crate/Cargo.toml"
        
        # Update any internal dependency versions
        sed -i.bak "s/raz-common = { path = \"[^\"]*\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-common = { path = \"..\/raz-common\", version = \"$NEW_VERSION\" }/g" "$crate/Cargo.toml"
        sed -i.bak "s/raz-config = { path = \"[^\"]*\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-config = { path = \"..\/raz-config\", version = \"$NEW_VERSION\" }/g" "$crate/Cargo.toml"
        sed -i.bak "s/raz-validation = { path = \"[^\"]*\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-validation = { path = \"..\/raz-validation\", version = \"$NEW_VERSION\" }/g" "$crate/Cargo.toml"
        sed -i.bak "s/raz-override = { path = \"[^\"]*\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-override = { path = \"..\/raz-override\", version = \"$NEW_VERSION\" }/g" "$crate/Cargo.toml"
        sed -i.bak "s/raz-core = { path = \"[^\"]*\", version = \"[0-9]\+\.[0-9]\+\.[0-9]\+\" }/raz-core = { path = \"..\/..\/raz-core\", version = \"$NEW_VERSION\" }/g" "$crate/Cargo.toml"
        
        # Clean up backup files
        rm -f "$crate/Cargo.toml.bak"
        
        print_success "Updated $crate"
    else
        print_warning "$crate/Cargo.toml not found, skipping"
    fi
done

# Update VS Code extension version
print_step "Updating VS Code extension version"
if [[ -f "raz-adapters/vscode/package.json" ]]; then
    # Use a more precise sed command for package.json
    sed -i.bak "s/\"version\": \"[0-9]\+\.[0-9]\+\.[0-9]\+\"/\"version\": \"$NEW_VERSION\"/" "raz-adapters/vscode/package.json"
    rm -f "raz-adapters/vscode/package.json.bak"
    print_success "Updated VS Code extension version"
else
    print_warning "VS Code package.json not found, skipping"
fi

# Clean up main Cargo.toml backup
rm -f "Cargo.toml.bak"

print_step "Running cargo check to validate changes"
if ! cargo check --workspace --all --all-features --all-targets; then
    print_error "Cargo check failed. Rolling back changes."
    git checkout -- .
    exit 1
fi

print_success "All version updates completed successfully"

# Commit version changes
print_step "Committing version changes"
git add .
git commit -m "chore: bump version to $NEW_VERSION"

print_step "Creating git tag"
git tag -a "v$NEW_VERSION" -m "Release version $NEW_VERSION"

if [[ $IS_PRE_RELEASE == true ]]; then
    print_step "Skipping crates.io publishing for pre-release version"
    print_warning "Pre-release versions are not published to crates.io"
    print_warning "Only published to GitHub releases for testing"
else
    print_step "Publishing crates to crates.io"

    # Publish crates in dependency order
    for crate in "${CRATES[@]}"; do
        print_step "Publishing $crate"
        
        if [[ -f "$crate/Cargo.toml" ]]; then
            cd "$crate"
            
            # Check if already published
            if cargo search --limit 1 "$(basename $crate)" | grep -q "$NEW_VERSION"; then
                print_warning "$crate $NEW_VERSION already published, skipping"
            else
                if cargo publish --dry-run; then
                    print_step "Dry run successful, publishing $crate"
                    cargo publish
                    print_success "Published $crate $NEW_VERSION"
                    
                    # Wait a bit for the registry to update
                    sleep 10
                else
                    print_error "Dry run failed for $crate"
                    cd ..
                    exit 1
                fi
            fi
            
            cd ..
        else
            print_warning "$crate/Cargo.toml not found, skipping publish"
        fi
    done
fi

# Handle VS Code extension
if [[ $IS_PRE_RELEASE == true ]]; then
    print_step "Publishing VS Code extension as PRE-RELEASE"
    if [[ -f "raz-adapters/vscode/package.json" ]]; then
        cd "raz-adapters/vscode"
        
        # Check if vsce is installed
        if ! command -v vsce &> /dev/null; then
            print_error "vsce is not installed. Install it with: npm install -g @vscode/vsce"
            cd ../..
            exit 1
        fi
        
        # Install dependencies and build
        print_step "Installing VS Code extension dependencies"
        npm install
        
        print_step "Building VS Code extension"
        npm run bundle
        
        print_step "Publishing VS Code extension to marketplace as PRE-RELEASE"
        if vsce publish --pat "$VSCODE_PAT" --pre-release; then
            print_success "VS Code extension published as PRE-RELEASE successfully"
        else
            print_error "Failed to publish VS Code extension as pre-release"
            print_warning "Make sure VSCODE_PAT environment variable is set"
            cd ../..
            exit 1
        fi
        
        cd ../..
    else
        print_warning "VS Code package.json not found, skipping extension publish"
    fi
else
    print_step "Publishing VS Code extension"
    if [[ -f "raz-adapters/vscode/package.json" ]]; then
        cd "raz-adapters/vscode"
        
        # Check if vsce is installed
        if ! command -v vsce &> /dev/null; then
            print_error "vsce is not installed. Install it with: npm install -g @vscode/vsce"
            cd ../..
            exit 1
        fi
        
        # Install dependencies and build
        print_step "Installing VS Code extension dependencies"
        npm install
        
        print_step "Building VS Code extension"
        npm run bundle
        
        print_step "Publishing VS Code extension to marketplace"
        if vsce publish --pat "$VSCODE_PAT"; then
            print_success "VS Code extension published successfully"
        else
            print_error "Failed to publish VS Code extension"
            print_warning "Make sure VSCODE_PAT environment variable is set"
            cd ../..
            exit 1
        fi
        
        cd ../..
    else
        print_warning "VS Code package.json not found, skipping extension publish"
    fi
fi

print_step "Pushing git changes and tags"
git push origin main
git push origin "v$NEW_VERSION"

if [[ $IS_PRE_RELEASE == true ]]; then
    print_success "🎉 PRE-RELEASE $NEW_VERSION completed successfully!"
    echo
    echo "Summary:"
    echo "- Updated all crate versions to $NEW_VERSION"
    echo "- ⚠️  SKIPPED crates.io publishing (pre-release)"
    echo "- Updated VS Code extension to $NEW_VERSION"
    echo "- Published VS Code extension to marketplace as PRE-RELEASE"
    echo "- Created git tag v$NEW_VERSION and pushed to repository"
    echo "- 🤖 CI Release workflow triggered by tag push"
    echo
    echo "Next steps:"
    echo "1. ⚠️  Crates NOT published to crates.io (pre-release)"
    echo "2. Check VS Code marketplace for the PRE-RELEASE extension"
    echo "3. Monitor GitHub Actions for release artifacts build"
    echo "4. Check GitHub releases for cross-platform binaries"
    echo "5. Test the pre-release version thoroughly"
    echo "6. If testing successful, promote to stable with: ./scripts/release.sh $NEW_VERSION"
else
    print_success "🎉 Release $NEW_VERSION completed successfully!"
    echo
    echo "Summary:"
    echo "- Updated all crate versions to $NEW_VERSION"
    echo "- Published all crates to crates.io"
    echo "- Updated VS Code extension to $NEW_VERSION"
    echo "- Published VS Code extension to marketplace"
    echo "- Created git tag v$NEW_VERSION and pushed to repository"
    echo "- 🤖 CI Release workflow triggered by tag push"
    echo
    echo "Next steps:"
    echo "1. Check https://crates.io/crates/raz-cli for the new version"
    echo "2. Check VS Code marketplace for the extension update"
    echo "3. Monitor GitHub Actions for release artifacts build"
    echo "4. Check GitHub releases for cross-platform binaries"
    echo "5. Update any documentation that references the old version"
fi