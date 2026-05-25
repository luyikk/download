#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

/// Download handler context
struct DownloadHandler;

extern "C" {

DownloadHandler *durl_create(uint32_t thread_count);

/// # Safety
/// free DownloadHandler
void durl_release(DownloadHandler *handler);

/// clean key money
void durl_clean(DownloadHandler *handler, uint64_t key);

/// # Safety
/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
uint64_t durl_start(DownloadHandler *handler,
                    const char *url,
                    const char *path,
                    uint64_t task,
                    uint64_t block);

/// # Safety
/// start download with a custom output file name
uint64_t durl_start_file_name(DownloadHandler *handler,
                              const char *url,
                              const char *path,
                              const char *file_name,
                              uint64_t task,
                              uint64_t block);

/// get download is start
bool durl_is_downloading(DownloadHandler *handler, uint64_t key);

bool durl_is_downloading_finish(const DownloadHandler *handler, uint64_t key);

void durl_suspend(const DownloadHandler *handler, uint64_t key);

void durl_restart(const DownloadHandler *handler, uint64_t key);

/// # Safety
/// get temp download path (.dd), returns copied c-string length
uint32_t durl_get_save_file_path(const DownloadHandler *handler, uint64_t key, char *msg);

/// # Safety
/// get final output file path, returns copied c-string length
uint32_t durl_get_real_file_path(const DownloadHandler *handler, uint64_t key, char *msg);

/// get state
/// if error return error msg len
uint32_t durl_get_state(const DownloadHandler *handler,
                        uint64_t key,
                        uint64_t *size,
                        uint64_t *down_size,
                        int32_t *err_code);

/// # Safety
/// get error msg string
void durl_get_error_str(const DownloadHandler *handler, uint64_t key, char *msg);

} // extern "C"
