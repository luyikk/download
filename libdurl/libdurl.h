#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

/// Download handler context
struct DownloadHandler;

extern "C" {

DownloadHandler *rd_create();

/// # Safety
/// free DownloadHandler
void rd_release(DownloadHandler *handler);

/// # Safety
/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
void rd_start(DownloadHandler *handler,
              const char *url,
              const char *path,
              uint64_t task,
              uint64_t block);

/// get download is start
bool rd_is_downloading(const DownloadHandler *handler);

/// get state
/// if error return error msg len
uint32_t rd_get_state(const DownloadHandler *handler,
                      uint64_t *size,
                      uint64_t *down_size,
                      int32_t *err_code);

/// # Safety
/// get error msg string
void rd_get_error_str(const DownloadHandler *handler, char *msg);

} // extern "C"
