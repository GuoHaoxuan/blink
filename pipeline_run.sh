sqlite3 blink.db < sql/refresh_task.sql
./work
./extract
cp ./tgfs.json ../snapshot-stepping-visual/app/
cd ../snapshot-stepping-visual
git add .
git commit -m "Auto update tgfs.json at $(date '+%Y-%m-%d %H:%M:%S')"
git push origin main
