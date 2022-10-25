```text
vc link lib:
    openssl:
        Crypt32.lib
        ws2_32.lib
        Bcrypt.lib
        Userenv.lib
        Ntdll.lib
        Secur32.lib
        Ncrypt.lib
        libdurl.lib
        
    rustls:
        Bcrypt.lib
        ws2_32.lib
        Ntdll.lib
        libdurl.lib

android build example:
 
 export TARGET_AR=~/.NDK/arm/bin/arm-linux-androideabi-ar
 export TARGET_CC=~/.NDK/arm/bin/arm-linux-androideabi-clang
 cargo build --target armv7-linux-androideabi --release
```

vc example:
``` c++

#include <iostream>
#include <Windows.h>
#include "libdurl.h"

bool check(DownloadHandler* runtime, uint64_t key) {

    if (durl_is_downloading(runtime, key)) {
        uint64_t size = 0;
        uint64_t down_size = 0;
        int32_t error_code = 0;
        auto len = durl_get_state(runtime, key, &size, &down_size, &error_code);

        if (error_code != 0) {
            std::string s;
            s.resize(len);
            durl_get_error_str(runtime, key, (char*)s.data());
            std::cout << s << std::endl;
            durl_clean(runtime, key);
            return true;
        }
        else {
            std::cout << key << " size:" << size << " down size:" << down_size << std::endl;
            if (size == down_size) {
                std::cout << key << " download finish" << std::endl;
                durl_clean(runtime, key);
                return true;
            }
        }
    }
    else {
        std::cout << key << " download not run" << std::endl;
    }

    return false;
}


int main()
{
    auto runtime = durl_create(2);

    bool key1_finish = false;
    auto key1= durl_start(runtime, "https://u3dtestc.oss-ap-southeast-1.aliyuncs.com/test-ali-yun.zip", "d:/", 1,1024*1024);

    bool key2_finish = false;
    auto key2 = durl_start(runtime, "https://mya13-res.s3.ap-southeast-1.amazonaws.com/jjj_hotfix/fish/latest.zip", "d:/", 1, 1024 * 1024);

    for (;;) {

        if (!key1_finish&&check(runtime,key1)) {
            key1_finish = true;
        }

        if (!key2_finish && check(runtime, key2)) {
            key2_finish = true;
        }

        if (key1_finish && key2_finish)
            break;
       
        Sleep(50);        
    }

    durl_release(runtime);
}



```