1.检查如果当前软件编译成 linux 可执行文件后，是否会出现以下报错：
Hello@Hello-pc:~/下载/20260717-tool.tar等2个文件$ ./click_automation 
./click_automation: /lib/x86_64-linux-gnu/libstdc++.so.6: version `GLIBCXX_3.4.29' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libstdc++.so.6: version `GLIBCXX_3.4.30' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libstdc++.so.6: version `GLIBCXX_3.4.31' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libm.so.6: version `GLIBC_2.35' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.32' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.33' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.34' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found (required by ./click_automation)
./click_automation: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found (required by ./click_automation)

如果有，测试修复


定位参考：AGENTS.md
指导要求：Guidance.md