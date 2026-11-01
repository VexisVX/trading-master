cd "/run/media/vexis/New Volume/script/trading-manager"

# 1) Init
git init

# 2) Config (اگر قبلاً نکردی)
git config user.name "Your Name"
git config user.email "you@example.com"

# 3) Add all
git add .

# 4) ببین چه چیزی stage شده
git status --short | head -20

# 5) Commit
git commit -m "Initial commit"
