#!/bin/bash
git commit --amend --cleanup=verbatim -F clean_commit_msg.txt
git push -f origin feature/internal-ops-dashboard
git log -1 --pretty=format:%B
