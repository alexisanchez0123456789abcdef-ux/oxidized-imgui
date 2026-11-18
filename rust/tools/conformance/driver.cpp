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

static void DumpDrawListBody(const ImDrawList& list)
{
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

static void DumpDrawList(const char* scenario, const ImDrawList& list)
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario %s\n", scenario);
    DumpDrawListBody(list);
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

// font_metrics: the default font's baked metrics, glyph advances and CalcTextSize.
static void DumpFontMetrics()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario font_metrics\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    if (baked == NULL)
    {
        printf("no baked font\n");
        return;
    }

    printf("font size %08x ascent %08x descent %08x\n",
        Bits(baked->Size), Bits(baked->Ascent), Bits(baked->Descent));

    const char glyphs[5] = { 'H', 'i', 'e', 'l', 'o' };
    for (char c : glyphs)
    {
        ImFontGlyph* glyph = baked->FindGlyph((ImWchar)c);
        printf("glyph '%c' advance %08x\n", c, Bits(glyph ? glyph->AdvanceX : 0.0f));
    }

    struct Text
    {
        const char* text;
        float size;
    };
    const Text texts[4] = { { "", 13.0f }, { "Hello", 13.0f }, { "Hello", 26.0f }, { "Hi", 13.0f } };
    for (const Text& t : texts)
    {
        const ImVec2 measured = font->CalcTextSizeA(t.size, FLT_MAX, 0.0f, t.text);
        printf("text \"%s\" at %08x size %08x %08x\n", t.text, Bits(t.size),
            Bits(measured.x), Bits(measured.y));
    }
}

static void DumpList(const char* name, const ImDrawList& list)
{
    printf("%s cmds %d vtx %d idx %d\n", name, list.CmdBuffer.Size, list.VtxBuffer.Size, list.IdxBuffer.Size);
    for (int i = 0; i < list.CmdBuffer.Size; i++)
    {
        const ImDrawCmd& cmd = list.CmdBuffer[i];
        printf("  cmd %d clip %08x %08x %08x %08x elems %u\n", i,
            Bits(cmd.ClipRect.x), Bits(cmd.ClipRect.y), Bits(cmd.ClipRect.z), Bits(cmd.ClipRect.w),
            cmd.ElemCount);
    }
}

// clip_stack: the clip stack of a draw list and the command splits it causes.
static void DumpClipStack()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario clip_stack\n");

    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();
    {
        ImDrawList list(sd);
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        list.PushClipRect(ImVec2(70.0f, 70.0f), ImVec2(170.0f, 170.0f), false);
        list.PrimReserve(6, 4);
        list.PrimRect(ImVec2(60.0f, 60.0f), ImVec2(200.0f, 200.0f), COL);
        list.PopClipRect();
        list.PrimReserve(6, 4);
        list.PrimRect(ImVec2(210.0f, 210.0f), ImVec2(250.0f, 250.0f), COL);
        DumpList("clip_simple", list);
    }
    {
        ImDrawList list(sd);
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        list.PushClipRect(ImVec2(60.0f, 60.0f), ImVec2(200.0f, 200.0f), false);
        list.PushClipRect(ImVec2(70.0f, 70.0f), ImVec2(300.0f, 300.0f), true);
        list.PrimReserve(6, 4);
        list.PrimRect(ImVec2(65.0f, 65.0f), ImVec2(290.0f, 290.0f), COL);
        list.PopClipRect();
        list.PrimReserve(6, 4);
        list.PrimRect(ImVec2(65.0f, 65.0f), ImVec2(190.0f, 190.0f), COL);
        list.PopClipRect();
        DumpList("clip_intersect", list);
    }
}

// Text dump: same shape as a draw list dump but without UVs, which depend on the atlas packer and
// are not reproducible across runs.
static void DumpTextBody(const ImDrawList& list)
{
    printf("uv omitted: atlas packing is not reproducible across runs\n");
    printf("cmd_count %d\n", list.CmdBuffer.Size);
    for (int i = 0; i < list.CmdBuffer.Size; i++)
    {
        const ImDrawCmd& cmd = list.CmdBuffer[i];
        printf("cmd %d clip %08x %08x %08x %08x elems %u\n", i,
            Bits(cmd.ClipRect.x), Bits(cmd.ClipRect.y), Bits(cmd.ClipRect.z), Bits(cmd.ClipRect.w),
            cmd.ElemCount);
    }
    printf("vtx_count %d\n", list.VtxBuffer.Size);
    for (int i = 0; i < list.VtxBuffer.Size; i++)
    {
        const ImDrawVert& v = list.VtxBuffer[i];
        printf("vtx %d pos %08x %08x col %08x\n", i, Bits(v.pos.x), Bits(v.pos.y), v.col);
    }
    printf("idx_count %d\n", list.IdxBuffer.Size);
    for (int i = 0; i < list.IdxBuffer.Size; i++)
        printf("idx %u\n", (unsigned int)list.IdxBuffer[i]);
}

// text_draw: two glyph quads from the default font, using the legacy atlas build.
static void DumpTextDraw()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario text_draw\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    // The atlas layout depends on when (and in which order) glyphs are baked, so bake a fixed set
    // before starting the frame. This makes the UVs reproducible and turns them into pinned inputs.
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[5] = { 'H', 'i', 'e', 'l', 'o' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);

    ImGui::NewFrame();
    {
        ImDrawList list(ImGui::GetDrawListSharedData());
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        font->RenderText(&list, 13.0f, ImVec2(10.0f, 20.0f), IM_COL32(255, 255, 255, 255),
            ImVec4(0.0f, 0.0f, 1280.0f, 720.0f), "Hi", NULL, 0.0f);
        DumpTextBody(list);
    }
    ImGui::EndFrame();
}

// button: one button, six frames of mouse input.
static void DumpButton()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario button\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImGui::NewFrame();
    ImGui::EndFrame();

    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    struct Frame
    {
        float x, y;
        bool down;
    };
    const Frame frames[6] = {
        { 0.0f, 0.0f, false },  { 96.0f, 89.0f, false }, { 96.0f, 89.0f, true },
        { 96.0f, 89.0f, true }, { 96.0f, 89.0f, false }, { 96.0f, 89.0f, false },
    };

    for (int i = 0; i < 6; i++)
    {
        io.DeltaTime = 0.1f;
        io.MousePos = ImVec2(frames[i].x, frames[i].y);
        io.MouseDown[0] = frames[i].down;
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
        const bool ret = ImGui::Button("Hi");
        const ImVec2 mn = ImGui::GetItemRectMin();
        const ImVec2 mx = ImGui::GetItemRectMax();
        const bool hovered = ImGui::IsItemHovered();
        const bool active = ImGui::IsItemActive();
        const ImVec2 cursor = ImGui::GetCursorScreenPos();
        printf("f%d pos %08x %08x down %d rect %08x %08x %08x %08x hov %d act %d ret %d cursor %08x %08x\n",
            i, Bits(frames[i].x), Bits(frames[i].y), frames[i].down ? 1 : 0,
            Bits(mn.x), Bits(mn.y), Bits(mx.x), Bits(mx.y),
            hovered ? 1 : 0, active ? 1 : 0, ret ? 1 : 0, Bits(cursor.x), Bits(cursor.y));
        ImGui::End();
        ImGui::EndFrame();
    }
}

// widgets_basic: a label's item rect and a checkbox over six frames.
static void DumpWidgets()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario widgets_basic\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImGui::NewFrame();
    ImGui::EndFrame();

    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;

    // Label.
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, flags);
    ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
    ImGui::Text("Hi");
    {
        const ImVec2 mn = ImGui::GetItemRectMin();
        const ImVec2 mx = ImGui::GetItemRectMax();
        const ImVec2 cursor = ImGui::GetCursorScreenPos();
        printf("label rect %08x %08x %08x %08x cursor %08x %08x\n",
            Bits(mn.x), Bits(mn.y), Bits(mx.x), Bits(mx.y), Bits(cursor.x), Bits(cursor.y));
    }
    ImGui::End();
    ImGui::EndFrame();

    // Checkbox.
    struct Frame
    {
        float x, y;
        bool down;
    };
    const Frame frames[6] = {
        { 0.0f, 0.0f, false },  { 98.0f, 89.0f, false }, { 98.0f, 89.0f, true },
        { 98.0f, 89.0f, false }, { 0.0f, 0.0f, false },  { 98.0f, 89.0f, true },
    };
    bool value = false;
    for (int i = 0; i < 6; i++)
    {
        io.DeltaTime = 0.1f;
        io.MousePos = ImVec2(frames[i].x, frames[i].y);
        io.MouseDown[0] = frames[i].down;
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
        const bool ret = ImGui::Checkbox("Hi", &value);
        const ImVec2 mn = ImGui::GetItemRectMin();
        const ImVec2 mx = ImGui::GetItemRectMax();
        const ImVec2 cursor = ImGui::GetCursorScreenPos();
        printf("cb%d rect %08x %08x %08x %08x hov %d act %d v %d ret %d cursor %08x %08x\n", i,
            Bits(mn.x), Bits(mn.y), Bits(mx.x), Bits(mx.y),
            ImGui::IsItemHovered() ? 1 : 0, ImGui::IsItemActive() ? 1 : 0, value ? 1 : 0,
            ret ? 1 : 0, Bits(cursor.x), Bits(cursor.y));
        ImGui::End();
        ImGui::EndFrame();
    }
}

// slider_basic: a float slider pressed, dragged and released.
static void DumpSlider()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario slider_basic\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImGui::NewFrame();
    ImGui::EndFrame();

    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    struct Frame
    {
        float x, y;
        bool down;
    };
    const Frame frames[5] = {
        { 0.0f, 0.0f, false },
        { 130.0f, 129.0f, false },
        { 130.0f, 129.0f, true },
        { 150.0f, 129.0f, true },
        { 150.0f, 129.0f, false },
    };
    float value = 5.0f;
    for (int i = 0; i < 5; i++)
    {
        io.DeltaTime = 0.1f;
        io.MousePos = ImVec2(frames[i].x, frames[i].y);
        io.MouseDown[0] = frames[i].down;
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImGui::SetCursorScreenPos(ImVec2(80.0f, 120.0f));
        ImGui::SetNextItemWidth(100.0f);
        const bool ret = ImGui::SliderFloat("S", &value, 0.0f, 10.0f);
        const ImVec2 mn = ImGui::GetItemRectMin();
        const ImVec2 mx = ImGui::GetItemRectMax();
        const ImVec2 cursor = ImGui::GetCursorScreenPos();
        printf("sl%d rect %08x %08x %08x %08x hov %d act %d v %08x ret %d cursor %08x %08x\n", i,
            Bits(mn.x), Bits(mn.y), Bits(mx.x), Bits(mx.y),
            ImGui::IsItemHovered() ? 1 : 0, ImGui::IsItemActive() ? 1 : 0, Bits(value),
            ret ? 1 : 0, Bits(cursor.x), Bits(cursor.y));
        ImGui::End();
        ImGui::EndFrame();
    }
}

// style_defaults: the default style's colours, roundings and border sizes.
static void DumpStyleDefaults()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario style_defaults\n");

    ImGuiStyle& style = ImGui::GetStyle();
    const ImGuiCol cols[17] = {
        ImGuiCol_Text, ImGuiCol_TextDisabled, ImGuiCol_WindowBg, ImGuiCol_FrameBg,
        ImGuiCol_FrameBgHovered, ImGuiCol_FrameBgActive, ImGuiCol_Button, ImGuiCol_ButtonHovered,
        ImGuiCol_ButtonActive, ImGuiCol_CheckMark, ImGuiCol_SliderGrab, ImGuiCol_SliderGrabActive,
        ImGuiCol_Border, ImGuiCol_Separator, ImGuiCol_Header, ImGuiCol_HeaderHovered,
        ImGuiCol_HeaderActive,
    };
    const char* names[17] = {
        "Text", "TextDisabled", "WindowBg", "FrameBg", "FrameBgHovered", "FrameBgActive",
        "Button", "ButtonHovered", "ButtonActive", "CheckMark", "SliderGrab", "SliderGrabActive",
        "Border", "Separator", "Header", "HeaderHovered", "HeaderActive",
    };
    for (int i = 0; i < 17; i++)
        printf("col %s %08x\n", names[i], ImGui::GetColorU32(cols[i]));

    printf("round frame %08x window %08x grab %08x child %08x popup %08x scrollbar %08x tab %08x\n",
        Bits(style.FrameRounding), Bits(style.WindowRounding), Bits(style.GrabRounding),
        Bits(style.ChildRounding), Bits(style.PopupRounding), Bits(style.ScrollbarRounding),
        Bits(style.TabRounding));
    printf("border window %08x frame %08x child %08x popup %08x\n",
        Bits(style.WindowBorderSize), Bits(style.FrameBorderSize), Bits(style.ChildBorderSize),
        Bits(style.PopupBorderSize));
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
    if (strcmp(name, "font_metrics") == 0)
    {
        DumpFontMetrics();
        ImGui::DestroyContext();
        return 0;
    }

    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    if (strcmp(name, "text_draw") == 0)
    {
        DumpTextDraw();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "button") == 0)
    {
        DumpButton();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "widgets_basic") == 0)
    {
        DumpWidgets();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "slider_basic") == 0)
    {
        DumpSlider();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "style_defaults") == 0)
    {
        DumpStyleDefaults();
        ImGui::DestroyContext();
        return 0;
    }

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
    if (strcmp(name, "clip_stack") == 0)
    {
        ImGui::NewFrame();
        DumpClipStack();
        ImGui::EndFrame();
        ImGui::DestroyContext();
        return 0;
    }

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
