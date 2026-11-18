// Reference probe: prints what Dear ImGui actually does, so the Rust tests have a contract.
//
// Build and run with build.bat (x86_64 MSVC, from rust/tools/oracle).
// When you add a case to the Rust tests, add it here first.

#include "imgui_internal.h"
#include <cstdio>
#include <cstddef>

int main()
{
    printf("version %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("size ImVec2 %zu\n", sizeof(ImVec2));
    printf("size ImDrawVert %zu\n", sizeof(ImDrawVert));
    printf("size ImDrawCmd %zu\n", sizeof(ImDrawCmd));
    printf("size ImDrawIdx %zu\n", sizeof(ImDrawIdx));
    printf("size ImGuiID %zu\n", sizeof(ImGuiID));
    printf("off ImDrawCmd ClipRect %zu TexRef %zu VtxOffset %zu IdxOffset %zu ElemCount %zu UserCallback %zu\n",
        offsetof(ImDrawCmd, ClipRect), offsetof(ImDrawCmd, TexRef), offsetof(ImDrawCmd, VtxOffset),
        offsetof(ImDrawCmd, IdxOffset), offsetof(ImDrawCmd, ElemCount), offsetof(ImDrawCmd, UserCallback));
    printf("off ImDrawVert pos %zu uv %zu col %zu\n",
        offsetof(ImDrawVert, pos), offsetof(ImDrawVert, uv), offsetof(ImDrawVert, col));

    struct Case { ImGuiID seed; const char* s; };
    Case cases[] = {
        {0u, ""}, {0u, "Hello"}, {0u, "Window"}, {0u, "##DragDropCancelHandler"},
        {0u, "label###id"}, {0u, "a###b###c"}, {0u, "###"}, {12345u, "Hello"},
        {0xFFFFFFFFu, "abc"}, {0u, "caf\xC3\xA9"}, {0u, "\xF0\x9F\x98\x80"}, {0x11111111u, "ViewportDefault"}
    };
    for (const Case& c : cases)
        printf("hash %u \"%s\" %u\n", c.seed, c.s, ImHashStr(c.s, 0, c.seed));
    return 0;
}
