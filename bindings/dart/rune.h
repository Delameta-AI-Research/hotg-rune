#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Error Error;

typedef struct Image Image;

/**
 * The Rune runtime.
 */
typedef struct Runtime Runtime;

const char *rune_error_to_string(struct Error *error);

const char *rune_error_backtrace(struct Error *error);

void rune_error_free(struct Error *error);

struct Image *rune_image_new(void);

struct Image *rune_image_new_with_defaults(void);

struct Error *rune_image_register_model_handler(struct Image *image,
                                                const char *mimetype,
                                                uintptr_t mimetype_len,
                                                void *user_data,
                                                void (*create_model)(void*),
                                                void (*free)(void*));

struct Error *rune_image_register_capability_handler(struct Image *image,
                                                     uint32_t capability_type,
                                                     void *user_data,
                                                     void (*create_capability)(void*),
                                                     void (*free)(void*));

struct Error *rune_image_register_output_handler(struct Image *image,
                                                 uint32_t output_type,
                                                 void *user_data,
                                                 void (*create_output)(void*),
                                                 void (*free)(void*));

void rune_image_free(struct Image *img);

struct Error *rune_runtime_create(const char *wasm,
                                  uintptr_t wasm_len,
                                  struct Image *image,
                                  struct Runtime **runtime);

struct Error *rune_runtime_call(struct Runtime *runtime);

void rune_runtime_free(struct Runtime *runtime);
