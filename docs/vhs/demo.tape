# DialogDetective demo
# Run with: vhs demo.tape

Output demo.webm
Output demo.mp4

Set Shell "zsh"
Set FontFamily "JetBrainsMono NFM SemiBold"
Set FontSize 32
Set Width 1800
Set Height 800
Set Theme { "name": "DialogDetective", "black": "#0f0f14", "red": "#ef4444", "green": "#22c55e", "yellow": "#eab308", "blue": "#0ea5e9", "magenta": "#a78bfa", "cyan": "#06b6d4", "white": "#e5e5e5", "brightBlack": "#9ca3af", "brightRed": "#f87171", "brightGreen": "#4ade80", "brightYellow": "#facc15", "brightBlue": "#38bdf8", "brightMagenta": "#c4b5fd", "brightCyan": "#22d3ee", "brightWhite": "#ffffff", "background": "#0f0f14", "foreground": "#e5e5e5", "selection": "#1f1f2e", "cursor": "#0ea5e9" }
Set Padding 20

# Hidden setup - cd to demo directory and create alias
Set TypingSpeed 0
Hide
Type "cd /tmp/dialogdetective-demo"
Enter
Sleep 10ms
Type "alias dialog_detective='./mock_dialog_detective.sh'"
Enter
Sleep 10ms
Type "PROMPT='%F{#0ea5e9}> %f'"
Enter
Sleep 10ms
Type "clear"
Enter
Sleep 30ms
Show
Set TypingSpeed 50ms

# == VISIBLE RECORDING STARTS ==

Sleep 500ms

# Run dialog_detective
Type `dialog_detective ./videos "The Flash" --season 5`
Enter

# Wait for the mock script to complete (should take about 30 seconds)
Sleep 30s

# Brief pause at the end to show results
Sleep 3s
