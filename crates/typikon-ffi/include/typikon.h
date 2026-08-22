#ifndef TYPIKON_H
#define TYPIKON_H

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Both inputs must point to NUL-terminated UTF-8 JSON strings. The returned
 * string is owned by typikon-engine and must be released with
 * typikon_string_free. The response always conforms to
 * typikon.ffi-response/v0.1.
 */
char *typikon_compile(const char *resource_bundle_json, const char *request_json);

/* Releases a non-NULL string returned by typikon_compile. */
void typikon_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
