// Reference driver: prints the draw-list dump for one scenario, using the upstream sources.
//
// The Rust side must reproduce this output byte for byte; `gen-golden.ps1` builds this file into
// the oracle and stores the output as a golden test. Floats are printed as raw bit patterns so
// the comparison never depends on decimal formatting.
//
// Usage: driver.exe <scenario>

#include "imgui_internal.h"
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <io.h>

static const ImU32 COL = IM_COL32(0, 0, 255, 255);

static unsigned int Bits(float f)
{
    unsigned int u;
    memcpy(&u, &f, sizeof(u));
    return u;
}

static void Dump(const char* scenario, const ImDrawList& list)
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario %s\n", scenario);
    // The white-pixel UV comes from the font atlas: it is an input to the geometry, so the dump
    // records it and the Rust side pins the same value.
    printf("white_uv %08x %08x\n",
        Bits(list._Data->TexUvWhitePixel.x), Bits(list._Data->TexUvWhitePixel.y));

    printf("cmd_count %d\n", list.CmdBuffer.Size);
    for (int i = 0; i < list.CmdBuffer.Size; i++)
    {
        const ImDrawCmd& c = list.CmdBuffer[i];
        printf("cmd %d clip %08x %08x %08x %08x elems %u vtxoff %u idxoff %u\n", i,
            Bits(c.ClipRect.x), Bits(c.ClipRect.y), Bits(c.ClipRect.z), Bits(c.ClipRect.w),
            c.ElemCount, c.VtxOffset, c.IdxOffset);
    }

    printf("vtx_count %d\n", list.VtxBuffer.Size);
    for (int i = 0; i < list.VtxBuffer.Size; i++)
    {
        const ImDrawVert& v = list.VtxBuffer[i];
        printf("vtx %d pos %08x %08x uv %08x %08x col %08x\n", i,
            Bits(v.pos.x), Bits(v.pos.y), Bits(v.uv.x), Bits(v.uv.y), v.col);
    }

    printf("idx_count %d\n", list.IdxBuffer.Size);
    for (int i = 0; i < list.IdxBuffer.Size; i++)
        printf("idx %u\n", (unsigned int)list.IdxBuffer[i]);
}

// Explicit UVs: no atlas involved.
static void ScenePrimRectUV(ImDrawList& list)
{
    list.PrimReserve(6, 4);
    list.PrimRectUV(
        ImVec2(10.0f, 20.0f), ImVec2(30.0f, 50.0f),
        ImVec2(0.25f, 0.5f), ImVec2(0.75f, 1.0f), COL);
}

// White-pixel UV from the atlas.
static void ScenePrimRect(ImDrawList& list)
{
    list.PrimReserve(6, 4);
    list.PrimRect(ImVec2(10.0f, 20.0f), ImVec2(30.0f, 50.0f), COL);
}

// Public API, pixel-aligned path: upstream reduces this to PrimRect.
static void SceneRectFilled(ImDrawList& list)
{
    list.AddRectFilled(ImVec2(10.0f, 20.0f), ImVec2(30.0f, 50.0f), COL);
}

// Public API, default inside stroke, one pixel thick.
static void SceneLineH(ImDrawList& list)
{
    list.AddLineH(10.0f, 30.0f, 20.0f, COL, 1.0f);
}

int main(int argc, char** argv)
{
    if (argc < 2)
    {
        fprintf(stderr, "usage: driver <scenario>\n");
        return 2;
    }
    const char* name = argv[1];

    void (*scene)(ImDrawList&) = NULL;
    if (strcmp(name, "prim_rect_uv") == 0) scene = ScenePrimRectUV;
    else if (strcmp(name, "prim_rect") == 0) scene = ScenePrimRect;
    else if (strcmp(name, "rect_filled") == 0) scene = SceneRectFilled;
    else if (strcmp(name, "line_h") == 0) scene = SceneLineH;
    if (scene == NULL)
    {
        fprintf(stderr, "unknown scenario: %s\n", name);
        return 2;
    }

    // Binary stdout: the dump must stay LF-only so it can be compared byte for byte.
    _setmode(_fileno(stdout), _O_BINARY);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.DisplaySize = ImVec2(1280.0f, 720.0f);
    // No renderer backend here: declare the modern texture contract so NewFrame does not require
    // the atlas to be built beforehand.
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::NewFrame();

    {
        ImDrawList list(ImGui::GetDrawListSharedData());
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        scene(list);
        Dump(name, list);
    }

    ImGui::EndFrame();
    ImGui::DestroyContext();
    return 0;
}
