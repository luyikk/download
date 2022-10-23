```text
vc link:
Crypt32.lib
ws2_32.lib
Bcrypt.lib
Userenv.lib
Ntdll.lib
Secur32.lib
Ncrypt.lib
libdurl.lib

android build:
 
 export TARGET_AR=~/.NDK/arm/bin/arm-linux-androideabi-ar
 export TARGET_CC=~/.NDK/arm/bin/arm-linux-androideabi-clang
 cargo build --target armv7-linux-androideabi --release
```

```c++
#include <iostream>
#include <Windows.h>
#include "libdurl.h"

int main()
{
    auto download = durl_create();
    durl_start(download, "https://mya13-res.s3.ap-southeast-1.amazonaws.com/jjj_hotfix/fish/latest.zip", "d:/", 10,1024*512);
    for (;;) {
        if(durl_is_downloading(download)){
            uint64_t size = 0;
            uint64_t down_size = 0;
            int32_t error_code = 0;
            auto len= durl_get_state(download, &size, &down_size, &error_code);

            if (error_code != 0) {
                std::string s;
                s.resize(len);
                durl_get_error_str(download, (char*)s.data());
                std::cout << s << std::endl;
                break;
            }
            else {
                std::cout << "size:" << size << "down size:" << down_size << std::endl;
                if(size == down_size){
                    std::cout << "download finish" << std::endl;
                    break;
                }
                
            }
        }
        else {
            std::cout << "download not run" << std::endl;
        }
       
        Sleep(50);        
    }
    durl_release(download);
}
```