# 🔧 Fix: "Carbide Provider is Damaged" Error on macOS

## The Problem

When trying to install or open the Carbide Provider app on macOS, you see:

```
"Carbide Provider" is damaged and can't be opened.
You should move it to the Trash.
```

## Why This Happens

**The app is NOT actually damaged!** This is macOS Gatekeeper - a security feature that blocks apps that aren't code-signed with an Apple Developer certificate ($99/year).

Since Carbide Provider is an open-source project built locally or distributed without Apple's signature, macOS blocks it by default.

## Quick Fix (Choose One)

### ✅ Option 1: Remove Quarantine Flag (Fastest)

Open Terminal and run:

```bash
sudo xattr -cr "/Applications/Carbide Provider.app"
```

Then open the app normally:

```bash
open "/Applications/Carbide Provider.app"
```

**Done!** The app should now launch without issues.

---

### ✅ Option 2: Right-Click to Open (No Terminal)

1. Open **Finder**
2. Go to **Applications**
3. Find **"Carbide Provider"**
4. **Right-click** (or Control-click) on the app
5. Select **"Open"**
6. Click **"Open"** again when the dialog appears

macOS will remember your choice and allow future launches.

---

### ✅ Option 3: System Settings (macOS 13+)

1. Try to open the app (it will be blocked)
2. Open **System Settings**
3. Go to **Privacy & Security**
4. Scroll down to **Security** section
5. Look for a message about "Carbide Provider was blocked"
6. Click **"Open Anyway"**
7. Enter your password when prompted

---

### ✅ Option 4: Build from Source (Most Secure)

If you prefer to compile it yourself:

```bash
cd /path/to/carbide-node
./build-gui.sh
cp -r "gui/src-tauri/target/release/bundle/macos/Carbide Provider.app" /Applications/
sudo xattr -cr "/Applications/Carbide Provider.app"
open "/Applications/Carbide Provider.app"
```

Locally built apps are automatically trusted by macOS.

---

## Why Not Just Code Sign?

Code signing requires:
- Apple Developer Program membership ($99/year)
- Managing certificates and provisioning profiles
- Notarization process for each release
- Yearly renewals

For an open-source project, this adds complexity. We're considering:
1. Community code signing pool
2. Homebrew distribution (auto-trusted)
3. Official signed releases for v2.0

## Still Having Issues?

### Check if Gatekeeper is Enabled

```bash
spctl --status
```

Should show: `assessments enabled`

### Temporarily Disable Gatekeeper (Not Recommended)

```bash
sudo spctl --master-disable
# Open the app
sudo spctl --master-enable  # Re-enable after
```

### Verify App Isn't Actually Corrupted

```bash
codesign -vv "/Applications/Carbide Provider.app"
```

If you see signing errors, that's expected (unsigned app). If you see file corruption errors, re-download or rebuild.

---

## Prevention for Next Time

After building with `build-gui.sh`, the script now automatically removes quarantine flags. If you still see the error:

1. The app was downloaded (not built locally)
2. You moved it from a quarantine location
3. macOS re-applied the flag

Just run the `xattr -cr` command again!

---

## Related Documentation

- Full troubleshooting: See **INSTALL.md** (section: "macOS App is Damaged Error")
- Build instructions: See **GUI_COMPLETE.md**
- Security best practices: See **README.md**

---

**TL;DR**: Run this one command and you're good:

```bash
sudo xattr -cr "/Applications/Carbide Provider.app"
```
