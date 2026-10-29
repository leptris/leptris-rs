#include <leptris.h>
#include <stdio.h>
#include <string.h>

static LeptrisDocument g_doc = 0;
static LeptrisXPathResult g_res = 0;

__attribute__((export_name("stage_parse")))
int stage_parse(const char* xml, int len) {
    LeptrisStatus st = LEPTRIS_OK;
    if (g_doc) { leptris_document_free(g_doc); g_doc = 0; }
    g_doc = leptris_parse_string(xml, (size_t)len, &st);
    if (!g_doc) return -(int)st - 1000;
    LeptrisElement root = leptris_document_root(g_doc);
    return root ? (int)leptris_element_child_count(root) : 0;
}

__attribute__((export_name("stage_eval")))
int stage_eval(const char* xpath) {
    if (!g_doc) return -1000;
    if (g_res) { leptris_xpath_result_free(g_res); g_res = 0; }
    g_res = leptris_xpath_eval(g_doc, NULL, xpath);
    if (!g_res) return -2000;
    return (int)leptris_xpath_result_count(g_res);
}

__attribute__((export_name("stage_string")))
int stage_string(char* out, int cap) {
    if (!g_res) return -1000;
    char* s = leptris_xpath_result_string(g_res);
    if (!s) { out[0] = '\0'; return -2000; }
    size_t l = strlen(s);
    if (l >= (size_t)cap) l = (size_t)cap - 1;
    memcpy(out, s, l);
    out[l] = '\0';
    leptris_free_string(s);
    return (int)l;
}

__attribute__((export_name("printf_probe")))
int printf_probe(char* out, int cap, const char* fmt, double v) {
    snprintf(out, (size_t)cap, fmt, v);
    return (int)strlen(out);
}

__attribute__((export_name("snprintf_direct")))
int snprintf_direct(char* out, int cap) {
    snprintf(out, (size_t)cap, "%f", 2.5);
    return (int)strlen(out);
}
