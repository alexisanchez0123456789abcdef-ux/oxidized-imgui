// Reference driver: prints the dump for one scenario, using the upstream sources.
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

static void P(const char* key, ImVec2 v)
{
    printf("%s %08x %08x\n", key, Bits(v.x), Bits(v.y));
}

static void DumpDrawList(const char* scenario, const ImDrawList& list)
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

// window_layout: style, a fixed-size window, two items, a same-line item, and the remembered
// position on the next frame. No text, no atlas, no scrolling.
static void DumpWindowLayout()
{
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;

    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_layout\n");

    ImGuiStyle& st = ImGui::GetStyle();
    printf("style wp %08x %08x is %08x %08x fp %08x %08x\n",
        Bits(st.WindowPadding.x), Bits(st.WindowPadding.y),
        Bits(st.ItemSpacing.x), Bits(st.ItemSpacing.y),
        Bits(st.FramePadding.x), Bits(st.FramePadding.y));

    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, flags);
    P("w0 pos", ImGui::GetWindowPos());
    P("w0 size", ImGui::GetWindowSize());
    P("w0 content", ImGui::GetContentRegionAvail());
    P("w0 cursor", ImGui::GetCursorScreenPos());

    ImGui::Dummy(ImVec2(100.0f, 20.0f));
    P("w0 item0 min", ImGui::GetItemRectMin());
    P("w0 item0 max", ImGui::GetItemRectMax());
    P("w0 cursor_after0", ImGui::GetCursorScreenPos());

    ImGui::SameLine();
    P("w0 sameline", ImGui::GetCursorScreenPos());

    ImGui::Dummy(ImVec2(50.0f, 10.0f));
    P("w0 item1 min", ImGui::GetItemRectMin());
    P("w0 item1 max", ImGui::GetItemRectMax());
    P("w0 cursor_after1", ImGui::GetCursorScreenPos());
    ImGui::End();
    ImGui::EndFrame();

    ImGui::NewFrame();
    ImGui::Begin("W", NULL, flags);
    P("w1 pos", ImGui::GetWindowPos());
    P("w1 size", ImGui::GetWindowSize());
    ImGui::End();
    ImGui::EndFrame();
}

// io_mouse_events: five frames of raw mouse input, printing what NewFrame derives from it.
static void DumpIoMouse()
{
    struct Frame
    {
        float x, y;
        bool down;
    };
    const Frame frames[5] = {
        { 10.5f, 20.5f, false },
        { 12.7f, 22.2f, true },
        { 14.0f, 24.0f, true },
        { 14.0f, 24.0f, false },
        { 14.0f, 24.0f, true },
    };

    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario io_mouse_events\n");

    ImGuiIO& io = ImGui::GetIO();
    for (int i = 0; i < 5; i++)
    {
        io.DeltaTime = 0.1f;
        io.MousePos = ImVec2(frames[i].x, frames[i].y);
        io.MouseDown[0] = frames[i].down;
        ImGui::NewFrame();
        const ImVec2 pos = io.MousePos; // UpdateMouseInputs rounded it
        printf("f%d pos %08x %08x delta %08x %08x clk %d rel %d count %d dur %08x\n", i,
            Bits(pos.x), Bits(pos.y), Bits(io.MouseDelta.x), Bits(io.MouseDelta.y),
            io.MouseClicked[0] ? 1 : 0, io.MouseReleased[0] ? 1 : 0,
            io.MouseClickedCount[0], Bits(io.MouseDownDuration[0]));
        ImGui::EndFrame();
    }
}

// io_keyboard_events: queued key events across frames, one duplicate, two different keys in one
// frame.
static void DumpIoKeyboard()
{
    struct KeyEv
    {
        ImGuiKey key;
        bool down;
    };
    struct KFrame
    {
        int count;
        KeyEv ev[2];
    };
    const KeyEv none = { ImGuiKey_None, false };
    const KFrame kframes[9] = {
        { 0, { none, none } },
        { 1, { { ImGuiKey_A, true }, none } },
        { 0, { none, none } },
        { 2, { { ImGuiKey_A, false }, { ImGuiKey_A, false } } },
        { 2, { { ImGuiKey_LeftCtrl, true }, { ImGuiKey_A, true } } },
        { 2, { { ImGuiKey_LeftCtrl, false }, { ImGuiKey_A, false } } },
        { 0, { none, none } },
        { 1, { { ImGuiMod_Ctrl, true }, none } },
        { 1, { { ImGuiMod_Ctrl, false }, none } },
    };

    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario io_keyboard_events\n");

    ImGuiIO& io = ImGui::GetIO();
    for (int i = 0; i < 9; i++)
    {
        io.DeltaTime = 0.1f;
        for (int e = 0; e < kframes[i].count; e++)
            io.AddKeyEvent(kframes[i].ev[e].key, kframes[i].ev[e].down);
        ImGui::NewFrame();

        const ImGuiKeyData* a = ImGui::GetKeyData(ImGuiKey_A);
        const ImGuiKeyData* ctrl = ImGui::GetKeyData(ImGuiKey_LeftCtrl);
        printf("f%d t %08x a_down %d a_dur %08x a_press %d ctrl_down %d ctrl_dur %08x ctrl_press %d mods %d%d%d%d\n",
            i, Bits(ImGui::GetTime()),
            a->Down ? 1 : 0, Bits(a->DownDuration), ImGui::IsKeyPressed(ImGuiKey_A, false) ? 1 : 0,
            ctrl->Down ? 1 : 0, Bits(ctrl->DownDuration), ImGui::IsKeyPressed(ImGuiKey_LeftCtrl, false) ? 1 : 0,
            io.KeyCtrl ? 1 : 0, io.KeyShift ? 1 : 0, io.KeyAlt ? 1 : 0, io.KeySuper ? 1 : 0);
        ImGui::EndFrame();
    }
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

    // Binary stdout: the dump must stay LF-only so it can be compared byte for byte.
    _setmode(_fileno(stdout), _O_BINARY);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.DisplaySize = ImVec2(1280.0f, 720.0f);
    // No renderer backend here: declare the modern texture contract so NewFrame does not require
    // the atlas to be built beforehand.
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    if (strcmp(name, "window_layout") == 0)
    {
        DumpWindowLayout();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "io_mouse_events") == 0)
    {
        DumpIoMouse();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "io_keyboard_events") == 0)
    {
        DumpIoKeyboard();
        ImGui::DestroyContext();
        return 0;
    }

    void (*scene)(ImDrawList&) = NULL;
    if (strcmp(name, "prim_rect_uv") == 0) scene = ScenePrimRectUV;
    else if (strcmp(name, "prim_rect") == 0) scene = ScenePrimRect;
    else if (strcmp(name, "rect_filled") == 0) scene = SceneRectFilled;
    else if (strcmp(name, "line_h") == 0) scene = SceneLineH;
    if (scene == NULL)
    {
        fprintf(stderr, "unknown scenario: %s\n", name);
        ImGui::DestroyContext();
        return 2;
    }

    ImGui::NewFrame();
    {
        ImDrawList list(ImGui::GetDrawListSharedData());
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        scene(list);
        DumpDrawList(name, list);
    }
    ImGui::EndFrame();
    ImGui::DestroyContext();
    return 0;
}
