# Render with: ./docs/render-demo.sh

Output docs/demo.webp

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 720
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms
Set PlaybackSpeed 1.0

Type "nix-tree ~/.nix-profile"
Sleep 500ms
Enter
# Skip closure load time.
Hide
Sleep 5s
Show
Sleep 1s

# user-environment -> home-manager-path -> packages
Right
Sleep 700ms
Down
Sleep 500ms
Right
Sleep 1s

Down@120ms 10
Sleep 1s

# why-depends
Type "w"
Sleep 2.5s
Escape
Sleep 500ms

# search
Type "/"
Sleep 400ms
Type "openssl"
Sleep 1.5s
Enter
Sleep 1.5s

# sort toggle
Type "s"
Sleep 1.5s

# help
Type "?"
Sleep 3s
Escape
Sleep 500ms

Type "q"
Sleep 500ms
