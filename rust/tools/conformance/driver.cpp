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

/// Optional path where a scenario can write the fixture the Rust side consumes.
static const char* g_fixture_path = NULL;

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

    // The atlas coordinates are only stable inside this process, so the fixture the Rust side
    // consumes is written by the same run that produced the golden.
    if (g_fixture_path != NULL)
    {
        if (FILE* f = fopen(g_fixture_path, "wb"))
        {
            fprintf(f, "font size %08x ascent %08x descent %08x\n", Bits(baked->Size),
                Bits(baked->Ascent), Bits(baked->Descent));
            for (char c : bake_order)
            {
                ImFontGlyph* g = baked->FindGlyph((ImWchar)c);
                fprintf(f, "g %d %08x %08x %08x %08x %08x %08x %08x %08x %08x\n", (int)c,
                    Bits(g->AdvanceX), Bits(g->U0), Bits(g->V0), Bits(g->U1), Bits(g->V1),
                    Bits(g->X0), Bits(g->Y0), Bits(g->X1), Bits(g->Y1));
            }
            fclose(f);
        }
    }

    ImGui::NewFrame();
    {
        ImDrawList list(ImGui::GetDrawListSharedData());
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = 0;
        font->RenderText(&list, 13.0f, ImVec2(10.0f, 20.0f), IM_COL32(255, 255, 255, 255),
            ImVec4(0.0f, 0.0f, 1280.0f, 720.0f), "Hi", NULL, 0.0f);
        DumpDrawListBody(list);
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

// Anti-aliased fill of a non-pixel-aligned rectangle: the miter fringe path.
static void SceneAaRect(ImDrawList& list)
{
    list.Flags = ImDrawListFlags_AntiAliasedFill;
    list.AddRectFilled(ImVec2(10.5f, 20.25f), ImVec2(60.5f, 50.75f), COL);
}

// aa_round_rect: a rounded rectangle, antialiased. Prints the arc points then the draw list.
static void DumpAaRoundRect()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario aa_round_rect\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::NewFrame();

    const ImVec2 a(10.5f, 20.25f);
    const ImVec2 b(60.5f, 50.75f);
    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();

    {
        ImDrawList path_list(sd);
        path_list._ResetForNewFrame();
        path_list._SetPixelDensity(1.0f);
        path_list.PathRect(a, b, 5.0f, ImDrawFlags_RoundAll);
        for (int i = 0; i < path_list._Path.Size; i++)
            printf("p %d %08x %08x\n", i, Bits(path_list._Path[i].x), Bits(path_list._Path[i].y));
    }
    {
        ImDrawList list(sd);
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.Flags = ImDrawListFlags_AntiAliasedFill;
        list.AddRectFilled(a, b, COL, 5.0f);
        DumpDrawListBody(list);
    }
    ImGui::EndFrame();
}

// Anti-aliased lines: two segments with different thickness.
static void SceneAaLine(ImDrawList& list)
{
    list.AddLine(ImVec2(10.0f, 10.0f), ImVec2(50.0f, 30.0f), COL, 1.0f);
    list.AddLine(ImVec2(10.0f, 10.0f), ImVec2(50.0f, 30.0f), COL, 2.0f);
}

// A convex polygon with a sharp apex: the miter exceeds the limit, so the corner is bevelled.
static void SceneAaBevel(ImDrawList& list)
{
    list.Flags = ImDrawListFlags_AntiAliasedFill;
    const ImVec2 points[3] = {
        ImVec2(10.5f, 50.25f),
        ImVec2(200.5f, 20.5f),
        ImVec2(200.5f, 80.5f),
    };
    list.AddConvexPolyFilled(points, 3, COL);
}

// arc_steps: the automatic corner sampling for a range of corner radii.
static void DumpArcSteps()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario arc_steps\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::NewFrame();
    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();
    for (int r = 1; r <= 14; r++)
    {
        ImDrawList list(sd);
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        list.PathRect(ImVec2(10.5f, 20.25f), ImVec2(200.5f, 150.75f), (float)r, ImDrawFlags_RoundAll);
        const int per_corner = list._Path.Size / 4;
        const int step = 12 / (per_corner - 1);
        printf("r %d rect_pts %d step %d\n", r, list._Path.Size, step);
    }
    ImGui::EndFrame();
}

// bezier_points: the auto-tessellated points of a cubic bezier.
static void DumpBezierPoints()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario bezier_points\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::NewFrame();
    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();
    ImDrawList list(sd);
    list._ResetForNewFrame();
    list._SetPixelDensity(1.0f);
    list._Path.push_back(ImVec2(10.0f, 10.0f));
    list.PathBezierCubicCurveTo(ImVec2(50.0f, 90.0f), ImVec2(130.0f, -20.0f), ImVec2(180.0f, 60.0f), 0);
    printf("n %d\n", list._Path.Size);
    for (int i = 0; i < list._Path.Size; i++)
        printf("p %d %08x %08x\n", i, Bits(list._Path[i].x), Bits(list._Path[i].y));
    ImGui::EndFrame();
}

// Open polyline with miter joins: a mitered body between two simple caps.
static void SceneAaPolyline(ImDrawList& list)
{
    // No AntiAliasedLines flag: the reference then takes its untextured stroke path, which is the
    // one this port implements (the textured line patterns live in the atlas).
    list.Flags = 0;
    const ImVec2 points[3] = {
        ImVec2(10.5f, 10.25f),
        ImVec2(60.5f, 45.75f),
        ImVec2(120.25f, 20.5f),
    };
    list.AddPolyline(points, 3, COL, 3.0f, ImDrawFlags_JoinMiter);
}

// Closed path with the inside stroke: a one-pixel border around a rectangle.
static void SceneAaBorder(ImDrawList& list)
{
    // No AntiAliasedLines flag: the untextured stroke path.
    list.Flags = 0;
    list.AddRect(ImVec2(10.5f, 10.25f), ImVec2(60.5f, 45.75f), COL, 0.0f, 0, 1.0f);
}

// Writes the glyph table the scenario used, so the Rust side can bake the same numbers. The atlas
// is only stable inside this process, so the fixture must come from the same run as the golden.
static void WriteGlyphFixture(ImFont* font, const char* bake_order, int bake_count)
{
    if (g_fixture_path == NULL)
        return;
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    FILE* f = fopen(g_fixture_path, "wb");
    if (f == NULL)
        return;
    fprintf(f, "font size %08x ascent %08x descent %08x\n", Bits(baked->Size), Bits(baked->Ascent),
        Bits(baked->Descent));
    for (int i = 0; i < bake_count; i++)
    {
        ImFontGlyph* g = baked->FindGlyph((ImWchar)bake_order[i]);
        fprintf(f, "g %d %08x %08x %08x %08x %08x %08x %08x %08x %08x\n", (int)bake_order[i],
            Bits(g->AdvanceX), Bits(g->U0), Bits(g->V0), Bits(g->U1), Bits(g->V1),
            Bits(g->X0), Bits(g->Y0), Bits(g->X1), Bits(g->Y1));
    }
    fclose(f);
}

// button_draw: a frame with a single button in a window without background, so the draw list holds
// only the widget.
static void DumpButtonDraw()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario button_draw\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[5] = { 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 5);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
    ImGui::Button("Hi");
    ImDrawList* dl = ImGui::GetWindowDrawList();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// window_bg: a window background with no border.
static void DumpWindowBg()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_bg\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::GetStyle().WindowBorderSize = 0.0f;
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("W", NULL, flags);
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImGui::End();
    DumpDrawListBody(*dl);
    ImGui::EndFrame();
}

// window_border: a window with its background and one-pixel border.
static void DumpWindowBorder()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_border\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("W", NULL, flags);
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImGui::End();
    DumpDrawListBody(*dl);
    ImGui::EndFrame();
}

// window_title: a titled window, so the title band and its text are part of the list.
static void DumpWindowTitle()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_title\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 6);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("W", NULL, flags);
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImGui::End();
    DumpDrawListBody(*dl);
    ImGui::EndFrame();
}

// Writes the atlas line-texture table the stroked scenarios sample. Like the glyph table, its
// coordinates are only stable inside this process, so it travels with the golden.
static void WriteLineUvFixture()
{
    if (g_fixture_path == NULL)
        return;
    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();
    FILE* f = fopen(g_fixture_path, "wb");
    if (f == NULL)
        return;
    const int count = IM_DRAWLIST_TEX_LINES_WIDTH_MAX + 1 + IM_DRAWLIST_TEX_LINES_DETAILED_WIDTH_COUNT;
    for (int n = 0; n < count; n++)
    {
        const ImVec4& e = sd->TexUvLines[n];
        fprintf(f, "line %d %08x %08x %08x %08x\n", n, Bits(e.x), Bits(e.y), Bits(e.z), Bits(e.w));
    }
    fclose(f);
}

// aa_strokes: the anti-aliased stroke path with the atlas line textures, which is the path a real
// window's draw list takes by default. Lines of several thicknesses, an open polyline with soft
// joins and AA ends, and a closed inside stroke.
static void DumpAaStrokes()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario aa_strokes\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    // The line table only reaches the shared data once a frame has selected the atlas font.
    ImGui::NewFrame();
    WriteLineUvFixture();
    ImGui::EndFrame();

    ImGui::NewFrame();
    {
        ImDrawList list(ImGui::GetDrawListSharedData());
        list._ResetForNewFrame();
        list._SetPixelDensity(1.0f);
        // No Flags override: the list keeps `ImDrawListSharedData::InitialDrawFlags`, which is what
        // a real window's draw list starts with (AA fill, AA lines, AA line ends, textured corners).
        printf("list_flags %08x\n", (unsigned int)list.Flags);

        list.AddLine(ImVec2(10.0f, 10.0f), ImVec2(50.0f, 30.0f), COL, 1.0f);
        list.AddLine(ImVec2(10.0f, 40.0f), ImVec2(50.0f, 60.0f), COL, 2.0f);
        list.AddLine(ImVec2(10.0f, 70.0f), ImVec2(50.0f, 90.0f), COL, 4.0f);

        const ImVec2 points[3] = {
            ImVec2(70.5f, 10.25f),
            ImVec2(120.5f, 45.75f),
            ImVec2(180.25f, 20.5f),
        };
        list.AddPolyline(points, 3, COL, 3.0f, 0);

        // The shape a window's collapse arrow takes: inside stroke, anti-aliased ends, soft join.
        const ImVec2 arrow[3] = {
            ImVec2(70.5f, 120.25f),
            ImVec2(75.5f, 125.75f),
            ImVec2(80.5f, 120.25f),
        };
        list.AddPolyline(arrow, 3, COL, 2.0f, ImDrawFlags_StrokeInside | ImDrawFlags_AALineEnds);

        // A corner sharp enough that the miter limit turns the join into a clipped bevel.
        const ImVec2 sharp[3] = {
            ImVec2(10.5f, 200.25f),
            ImVec2(200.5f, 200.75f),
            ImVec2(10.5f, 205.25f),
        };
        list.AddPolyline(sharp, 3, COL, 2.0f, ImDrawFlags_StrokeInside | ImDrawFlags_AALineEnds);

        list.AddRect(ImVec2(100.5f, 100.25f), ImVec2(180.5f, 150.75f), COL, 0.0f, 0, 2.0f);
        DumpDrawListBody(list);
    }
    ImGui::EndFrame();
}

// Writes the glyph table and the atlas line rows a window icon needs: the title is text, and both
// the collapse arrow and the close cross are strokes that sample the line textures.
static void WriteWindowIconsFixture(ImFont* font, const char* bake_order, int bake_count)
{
    if (g_fixture_path == NULL)
        return;
    FILE* f = fopen(g_fixture_path, "wb");
    if (f == NULL)
        return;
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    fprintf(f, "font size %08x ascent %08x descent %08x\n", Bits(baked->Size), Bits(baked->Ascent),
        Bits(baked->Descent));
    for (int i = 0; i < bake_count; i++)
    {
        ImFontGlyph* g = baked->FindGlyph((ImWchar)bake_order[i]);
        fprintf(f, "g %d %08x %08x %08x %08x %08x %08x %08x %08x %08x\n", (int)bake_order[i],
            Bits(g->AdvanceX), Bits(g->U0), Bits(g->V0), Bits(g->U1), Bits(g->V1),
            Bits(g->X0), Bits(g->Y0), Bits(g->X1), Bits(g->Y1));
    }
    ImDrawListSharedData* sd = ImGui::GetDrawListSharedData();
    const int count = IM_DRAWLIST_TEX_LINES_WIDTH_MAX + 1 + IM_DRAWLIST_TEX_LINES_DETAILED_WIDTH_COUNT;
    for (int n = 0; n < count; n++)
    {
        const ImVec4& e = sd->TexUvLines[n];
        fprintf(f, "line %d %08x %08x %08x %08x\n", n, Bits(e.x), Bits(e.y), Bits(e.z), Bits(e.w));
    }
    fclose(f);
}

// One frame of the icon window. The window does not know its title bar height during its first
// frame, so the scene settles before anything is dumped.
static bool FrameWindowIcons(float mx, float my, bool down, bool* open, ImDrawList** out_list,
    bool diagnostics, bool* out_collapsed)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar;
    const bool began = ImGui::Begin("W", open, flags);
    if (out_list != NULL)
        *out_list = ImGui::GetWindowDrawList();
    if (out_collapsed != NULL)
        *out_collapsed = ImGui::GetCurrentWindow()->Collapsed;
    if (diagnostics)
    {
        ImGuiWindow* window = ImGui::GetCurrentWindow();
        ImRect title_bar = window->TitleBarRect();
        fprintf(stderr, "diag title_h %08x rect %08x %08x %08x %08x\n", Bits(window->TitleBarHeight),
            Bits(title_bar.Min.x), Bits(title_bar.Min.y), Bits(title_bar.Max.x), Bits(title_bar.Max.y));
        fprintf(stderr, "diag font_size %08x frame_pad %08x %08x inner %08x menu_pos %d\n",
            Bits(ImGui::GetFontSize()), Bits(ImGui::GetStyle().FramePadding.x),
            Bits(ImGui::GetStyle().FramePadding.y), Bits(ImGui::GetStyle().ItemInnerSpacing.x),
            (int)ImGui::GetStyle().WindowMenuButtonPosition);
        fprintf(stderr, "diag title_align %08x %08x\n", Bits(ImGui::GetStyle().WindowTitleAlign.x),
            Bits(ImGui::GetStyle().WindowTitleAlign.y));
        const ImVec2 text_size = ImGui::CalcTextSize("W", NULL, true);
        fprintf(stderr, "diag text_size %08x %08x\n", Bits(text_size.x), Bits(text_size.y));
    }
    ImGui::End();
    ImGui::EndFrame();
    return began;
}

// window_icons: a titled window that carries both window icons (a close button because p_open is
// not null, a collapse arrow because NoCollapse is not set).
static void DumpWindowIcons()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_icons\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);

    bool open = true;
    const int settle_frames = 13;
    for (int i = 0; i < settle_frames; i++)
        FrameWindowIcons(0.0f, 0.0f, false, &open, NULL, false, NULL);
    // A frame to wire the atlas tables, then the fixture and the frame that is dumped.
    FrameWindowIcons(0.0f, 0.0f, false, &open, NULL, false, NULL);
    WriteWindowIconsFixture(font, bake_order, 6);

    ImDrawList* dl = NULL;
    FrameWindowIcons(0.0f, 0.0f, false, &open, &dl, true, NULL);
    if (dl != NULL)
        DumpDrawListBody(*dl);
}

// window_icons_states: the same window hovered, held, released inside and outside, collapsed and
// expanded. Each state is one dumped section, taken right after its frame.
static void DumpWindowIconsStates()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_icons_states\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);

    bool open = true;
    for (int i = 0; i < 13; i++)
        FrameWindowIcons(0.0f, 0.0f, false, &open, NULL, false, NULL);
    FrameWindowIcons(0.0f, 0.0f, false, &open, NULL, false, NULL);
    WriteWindowIconsFixture(font, bake_order, 6);

    // The two icon squares: the arrow at (65, 63) and the cross at (342, 63), 13x13 each.
    const float arrow_x = 70.0f, arrow_y = 69.0f;
    const float close_x = 348.0f, close_y = 69.0f;

    auto section = [&](const char* name, float mx, float my, bool down) {
        ImDrawList* dl = NULL;
        bool collapsed = false;
        const bool began = FrameWindowIcons(mx, my, down, &open, &dl, false, &collapsed);
        printf("state %s open %d collapsed %d began %d\n", name, open ? 1 : 0,
            collapsed ? 1 : 0, began ? 1 : 0);
        if (dl != NULL)
            DumpDrawListBody(*dl);
    };

    section("resting", 0.0f, 0.0f, false);
    section("hover_arrow", arrow_x, arrow_y, false);
    section("hover_close", close_x, close_y, false);
    section("held_close", close_x, close_y, true);
    section("released_close", close_x, close_y, false);

    // The same press, released away from the button: it must not close.
    open = true;
    section("held_close_again", close_x, close_y, true);
    section("released_outside", 100.0f, 150.0f, false);

    // Collapse with the arrow, then expand again.
    section("held_arrow", arrow_x, arrow_y, true);
    section("released_arrow", arrow_x, arrow_y, false);
    section("collapsed", 0.0f, 0.0f, false);
    section("collapsed_hover_arrow", arrow_x, arrow_y, false);
    section("held_arrow_collapsed", arrow_x, arrow_y, true);
    section("released_arrow_collapsed", arrow_x, arrow_y, false);
    section("expanded", 0.0f, 0.0f, false);
}

// window_icons_drag: the mouse grabs the collapse arrow and drags past the threshold, which moves
// the window (the reference's CollapseButton calls StartMouseMovingWindow). The window may move, so
// its position is only requested on the first frame.
static bool FrameWindowIconsDrag(float mx, float my, bool down, bool* open, ImDrawList** out_list,
    bool first_frame, bool* out_collapsed)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    if (first_frame)
    {
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    }
    // No NoMove: the collapse arrow drag has to be allowed to move the window.
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar;
    const bool began = ImGui::Begin("W", open, flags);
    if (out_list != NULL)
        *out_list = ImGui::GetWindowDrawList();
    if (out_collapsed != NULL)
        *out_collapsed = ImGui::GetCurrentWindow()->Collapsed;
    ImGui::End();
    ImGui::EndFrame();
    return began;
}

static void DumpWindowIconsDrag()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_icons_drag\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);

    bool open = true;
    bool first = true;
    for (int i = 0; i < 13; i++)
    {
        FrameWindowIconsDrag(0.0f, 0.0f, false, &open, NULL, first, NULL);
        first = false;
    }
    FrameWindowIconsDrag(0.0f, 0.0f, false, &open, NULL, false, NULL);
    WriteWindowIconsFixture(font, bake_order, 6);

    // The arrow square starts at (65, 63) and is 13x13, so the press lands at (70, 69).
    auto section = [&](const char* name, float mx, float my, bool down) {
        ImDrawList* dl = NULL;
        bool collapsed = false;
        const bool began =
            FrameWindowIconsDrag(mx, my, down, &open, &dl, false, &collapsed);
        ImGuiWindow* window = ImGui::FindWindowByName("W");
        const ImVec2 pos = window ? window->Pos : ImVec2(0, 0);
        printf("state %s pos %08x %08x open %d collapsed %d began %d down %d\n", name,
            Bits(pos.x), Bits(pos.y), open ? 1 : 0, collapsed ? 1 : 0, began ? 1 : 0, down ? 1 : 0);
        if (dl != NULL)
            DumpDrawListBody(*dl);
    };

    section("idle", 0.0f, 0.0f, false);
    section("hover_arrow", 70.0f, 69.0f, false);
    section("press", 70.0f, 69.0f, true);
    // 4 right and 1 down: squared distance 17, under the default threshold of 6 (36).
    section("below_threshold", 74.0f, 70.0f, true);
    // 20 right and 21 down: squared distance 841, past the threshold. The move starts here but the
    // position only changes on the next frame.
    section("drag_start", 90.0f, 90.0f, true);
    section("dragged", 110.0f, 100.0f, true);
    section("released", 110.0f, 100.0f, false);
    section("after_release", 0.0f, 0.0f, false);
}

// windows_order: two overlapping titled windows, and what a click does to the order they are
// rendered in. The dump is the renderer's view: the list of ImDrawList the backend would draw, in
// order, with the hovered window beside it.
static void FrameTwoWindows(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar;
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("A", NULL, flags);
    ImGui::End();
    ImGui::SetNextWindowPos(ImVec2(110.0f, 100.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("B", NULL, flags);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowOrderFrame(int frame, const char* tag)
{
    ImGui::Render();
    ImDrawData* dd = ImGui::GetDrawData();
    ImGuiContext& g = *GImGui;
    const char* hovered = g.HoveredWindow ? g.HoveredWindow->Name : "-";
    printf("frame %d %s hovered %s lists %d\n", frame, tag, hovered, dd->CmdListsCount);
    for (int i = 0; i < dd->CmdListsCount; i++)
    {
        const ImDrawList* list = dd->CmdLists[i];
        // The list has no name; identify it by the window whose list pointer it is.
        const char* name = "?";
        for (int w = 0; w < g.Windows.Size; w++)
            if (g.Windows[w]->DrawList == list)
                name = g.Windows[w]->Name;
        printf("list %d %s\n", i, name);
        DumpDrawListBody(*list);
    }
}

static void DumpWindowsOrder()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_order\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);

    // A = (60,60)-(360,260), B = (110,100)-(410,300).
    for (int i = 0; i < 6; i++)
        FrameTwoWindows(0.0f, 0.0f, false);
    WriteGlyphFixture(font, bake_order, 2);
    int frame = 0;

    FrameTwoWindows(0.0f, 0.0f, false);
    DumpWindowOrderFrame(frame++, "idle");
    // Inside A only.
    FrameTwoWindows(70.0f, 70.0f, true);
    DumpWindowOrderFrame(frame++, "press_a_only");
    FrameTwoWindows(70.0f, 70.0f, false);
    DumpWindowOrderFrame(frame++, "release_a_only");
    FrameTwoWindows(0.0f, 0.0f, false);
    DumpWindowOrderFrame(frame++, "idle_a_on_top");
    // Inside both: the top-most window takes the click and stays on top.
    FrameTwoWindows(200.0f, 150.0f, true);
    DumpWindowOrderFrame(frame++, "press_both");
    FrameTwoWindows(200.0f, 150.0f, false);
    DumpWindowOrderFrame(frame++, "release_both");
    // Inside B only: B comes to the front.
    FrameTwoWindows(400.0f, 290.0f, true);
    DumpWindowOrderFrame(frame++, "press_b_only");
    FrameTwoWindows(400.0f, 290.0f, false);
    DumpWindowOrderFrame(frame++, "release_b_only");
    FrameTwoWindows(0.0f, 0.0f, false);
    DumpWindowOrderFrame(frame++, "idle_b_on_top");
}

// windows_move: dragging a window by its title bar and by its body, and the clamp that keeps it
// reachable when it is dragged off the display. Only the positions are dumped: the geometry of a
// window is what windows_order already covers.
static void FrameMovableWindow(float mx, float my, bool down, bool first)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    if (first)
    {
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    }
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoScrollbar);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsMove()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_move\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    bool first = true;
    for (int i = 0; i < 6; i++)
    {
        FrameMovableWindow(0.0f, 0.0f, false, first);
        first = false;
    }

    auto section = [&](const char* tag, float mx, float my, bool down) {
        FrameMovableWindow(mx, my, down, false);
        ImGuiWindow* window = ImGui::FindWindowByName("W");
        printf("state %s pos %08x %08x\n", tag, Bits(window->Pos.x), Bits(window->Pos.y));
    };

    section("settled", 0.0f, 0.0f, false);
    // The title bar of W is y 60..79 at x 60..360.
    section("press_title", 100.0f, 65.0f, true);
    section("drag_title_1", 140.0f, 90.0f, true);
    section("drag_title_2", 200.0f, 120.0f, true);
    section("release_title", 200.0f, 120.0f, false);
    // And from the body: with ConfigWindowsMoveFromTitleBarOnly off, the whole window moves.
    section("press_body", 250.0f, 200.0f, true);
    section("drag_body", 280.0f, 220.0f, true);
    section("release_body", 280.0f, 220.0f, false);
    // Far up-left: clamped so a corner stays inside the padded work area.
    section("press_topleft", 300.0f, 240.0f, true);
    section("drag_to_origin", 0.0f, 0.0f, true);
    section("drag_off_screen", -400.0f, -400.0f, true);
    section("release_off_screen", -400.0f, -400.0f, false);
    // And far down-right.
    section("press_again", 0.0f, 0.0f, true);
    section("drag_far_right", 2000.0f, 2000.0f, true);
    section("release_far_right", 2000.0f, 2000.0f, false);
}

// window_scroll: a window without a scrollbar whose content is taller than the window, scrolled by
// API. Printing the scroll, its maximum and the interior rectangle makes a mismatch explainable.
static void FrameScrolledWindow(int scroll_y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    if (scroll_y >= 0)
        ImGui::SetNextWindowScroll(ImVec2(0.0f, (float)scroll_y));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar;
    ImGui::Begin("S", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    const ImVec2 start = ImGui::GetCursorScreenPos();
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    const ImVec2 after = ImGui::GetCursorScreenPos();
    ImGui::GetWindowDrawList()->AddRectFilled(
        ImVec2(start.x, after.y), ImVec2(start.x + 50.0f, after.y + 20.0f), IM_COL32(255, 0, 0, 255));
    printf("frame scroll %08x max %08x content %08x %08x inner %08x %08x %08x %08x start %08x %08x after %08x %08x\n",
        Bits(w->Scroll.y), Bits(w->ScrollMax.y), Bits(w->ContentSize.x), Bits(w->ContentSize.y),
        Bits(w->InnerRect.Min.x), Bits(w->InnerRect.Min.y), Bits(w->InnerRect.Max.x), Bits(w->InnerRect.Max.y),
        Bits(start.x), Bits(start.y), Bits(after.x), Bits(after.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowScroll()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_scroll\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    for (int i = 0; i < 6; i++)
        FrameScrolledWindow(-1);
    // A scroll inside the range: applied from that frame on.
    for (int i = 0; i < 2; i++)
        FrameScrolledWindow(40);
    // Far past the maximum: clamped to it.
    for (int i = 0; i < 2; i++)
        FrameScrolledWindow(10000);
}

// window_scrollbar: a window whose content does not fit, so the vertical scrollbar appears. Only
// the drawing is covered here: the mouse stays away, so the grab is neither hovered nor held.
static void FrameScrollbarWindow(int scroll_y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    if (scroll_y >= 0)
        ImGui::SetNextWindowScroll(ImVec2(0.0f, (float)scroll_y));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("S", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    const ImRect bar = ImGui::GetWindowScrollbarRect(w, ImGuiAxis_Y);
    printf("frame scrollbar %d scroll %08x max %08x inner %08x %08x %08x %08x bar %08x %08x %08x %08x\n",
        w->ScrollbarY ? 1 : 0, Bits(w->Scroll.y), Bits(w->ScrollMax.y), Bits(w->InnerRect.Min.x),
        Bits(w->InnerRect.Min.y), Bits(w->InnerRect.Max.x), Bits(w->InnerRect.Max.y), Bits(bar.Min.x),
        Bits(bar.Min.y), Bits(bar.Max.x), Bits(bar.Max.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowScrollbar()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_scrollbar\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // The first frame has no content size yet, so the bar only appears on the second.
    for (int i = 0; i < 3; i++)
        FrameScrollbarWindow(-1);
    // With the content scrolled the grab moves down.
    FrameScrollbarWindow(40);
    FrameScrollbarWindow(40);
}

// window_scrollbar_drag: grabbing the scrollbar and clicking the track. The grab sits at the top
// while the scroll is 0, so the press below it is an absolute seek.
static void FrameScrollbarDrag(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("S", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    printf("frame scroll %08x max %08x\n", Bits(w->Scroll.y), Bits(w->ScrollMax.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowScrollbarDrag()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_scrollbar_drag\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    for (int i = 0; i < 4; i++)
        FrameScrollbarDrag(0.0f, 0.0f, false);
    // The bar spans x 345..359; the grab is at the top of it (y 63..185).
    FrameScrollbarDrag(352.0f, 100.0f, false);  // hovered
    FrameScrollbarDrag(352.0f, 100.0f, true);   // held on the grab
    FrameScrollbarDrag(352.0f, 120.0f, true);   // dragged down
    FrameScrollbarDrag(352.0f, 140.0f, true);
    FrameScrollbarDrag(352.0f, 140.0f, false);  // released
    FrameScrollbarDrag(0.0f, 0.0f, false);
    // A click far below the grab: absolute seek to that point.
    FrameScrollbarDrag(352.0f, 230.0f, true);
    FrameScrollbarDrag(352.0f, 230.0f, false);
    FrameScrollbarDrag(0.0f, 0.0f, false);
    // And a click above it, which seeks back towards the top.
    FrameScrollbarDrag(352.0f, 70.0f, true);
    FrameScrollbarDrag(352.0f, 70.0f, false);
    FrameScrollbarDrag(0.0f, 0.0f, false);
}

// window_wheel: the mouse wheel over a window scrolls it. The target is set in NewFrame and applied
// by the Begin of the same frame, so the dump of that frame already shows the new scroll.
static void FrameWheelWindow(float mx, float my, float wheel_y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = false;
    if (wheel_y != 0.0f)
        io.AddMouseWheelEvent(0.0f, wheel_y);
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("S", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    printf("frame wheel %08x scroll %08x max %08x\n", Bits(io.MouseWheel), Bits(w->Scroll.y),
        Bits(w->ScrollMax.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowWheel()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_wheel\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // The mouse sits over the window body, away from the scrollbar.
    for (int i = 0; i < 4; i++)
        FrameWheelWindow(200.0f, 150.0f, 0.0f);
    // Two notches down, one frame each.
    FrameWheelWindow(200.0f, 150.0f, -1.0f);
    FrameWheelWindow(200.0f, 150.0f, -1.0f);
    // A frame without wheel keeps the scroll.
    FrameWheelWindow(200.0f, 150.0f, 0.0f);
    // Back up: the step is symmetric.
    FrameWheelWindow(200.0f, 150.0f, 1.0f);
    FrameWheelWindow(200.0f, 150.0f, 1.0f);
    // A half notch keeps the fractional part of the step.
    FrameWheelWindow(200.0f, 150.0f, 0.5f);
    FrameWheelWindow(200.0f, 150.0f, 0.0f);
}

// window_autosize: a window that takes the size its content needs. The size follows the content of
// the previous frame, which is why shrinking or growing takes one frame to show up.
static void FrameAutoSizeWindow(float content_h)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_AlwaysAutoResize | ImGuiWindowFlags_NoTitleBar |
        ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("A", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, content_h));
    printf("frame size %08x %08x sizefull %08x %08x inner %08x %08x %08x %08x content %08x %08x\n",
        Bits(w->Size.x), Bits(w->Size.y), Bits(w->SizeFull.x), Bits(w->SizeFull.y),
        Bits(w->InnerRect.Min.x), Bits(w->InnerRect.Min.y), Bits(w->InnerRect.Max.x),
        Bits(w->InnerRect.Max.y), Bits(w->ContentSize.x), Bits(w->ContentSize.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowAutoSize()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_autosize\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // A settled window: the size follows the content of the previous frame.
    for (int i = 0; i < 6; i++)
        FrameAutoSizeWindow(300.0f);
}

// window_autosize_change: an auto-sized window whose content changes. The size always follows the
// content of the previous frame, so both shrinking and growing take one frame to show up.
static void FrameAutoSizeChange(float content_h)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_AlwaysAutoResize | ImGuiWindowFlags_NoTitleBar |
        ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("A", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    printf("frame draw %08x size %08x %08x content %08x %08x inner %08x %08x %08x %08x\n",
        Bits(content_h), Bits(w->Size.x), Bits(w->Size.y), Bits(w->ContentSize.x),
        Bits(w->ContentSize.y), Bits(w->InnerRect.Min.x), Bits(w->InnerRect.Min.y),
        Bits(w->InnerRect.Max.x), Bits(w->InnerRect.Max.y));
    ImGui::Dummy(ImVec2(100.0f, content_h));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowAutoSizeChange()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_autosize_change\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    FrameAutoSizeChange(300.0f);
    FrameAutoSizeChange(300.0f);
    // The content shrinks: the size follows one frame later.
    FrameAutoSizeChange(50.0f);
    FrameAutoSizeChange(50.0f);
    // And grows: same one-frame lag.
    FrameAutoSizeChange(200.0f);
    FrameAutoSizeChange(200.0f);
}

// window_wheel_lock: the window the wheel is locked to keeps scrolling even when the mouse moves
// over another window, until it moves more than the drag threshold or the lock timer runs out.
//
// A = (60,60)-(360,260), B = (358,60)-(658,260): they overlap by two pixels, so a four pixel move
// takes the mouse from A alone to a point inside both, where B is on top and therefore hovered.
static void FrameWheelLock(float mx, float my, float wheel_y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = false;
    if (wheel_y != 0.0f)
        io.AddMouseWheelEvent(0.0f, wheel_y);
    ImGui::NewFrame();
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("A", NULL, flags);
    ImGuiWindow* wa = ImGui::GetCurrentWindow();
    const float scroll_a = wa->Scroll.y;
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    ImGui::End();
    ImGui::SetNextWindowPos(ImVec2(358.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("B", NULL, flags);
    ImGuiWindow* wb = ImGui::GetCurrentWindow();
    const float scroll_b = wb->Scroll.y;
    ImGui::Dummy(ImVec2(100.0f, 300.0f));
    ImGui::End();
    printf("frame wheel %08x a %08x b %08x hovered %s locked %s\n", Bits(wheel_y), Bits(scroll_a),
        Bits(scroll_b), GImGui->HoveredWindow ? GImGui->HoveredWindow->Name : "-",
        GImGui->WheelingWindow ? GImGui->WheelingWindow->Name : "-");
    ImGui::EndFrame();
}

static void DumpWindowWheelLock()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_wheel_lock\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // Settle with the mouse over A alone (x 355 is left of B's edge at 358).
    for (int i = 0; i < 4; i++)
        FrameWheelLock(355.0f, 150.0f, 0.0f);
    // A notch over A.
    FrameWheelLock(355.0f, 150.0f, -1.0f);
    // A four pixel move puts the mouse inside both, where B is on top and hovered.
    FrameWheelLock(359.0f, 150.0f, 0.0f);
    // The lock keeps the wheel on A even though B is hovered now.
    FrameWheelLock(359.0f, 150.0f, -1.0f);
    FrameWheelLock(359.0f, 150.0f, 0.0f);
    // A big move takes the mouse to B alone: the lock is released and B scrolls.
    FrameWheelLock(500.0f, 150.0f, 0.0f);
    FrameWheelLock(500.0f, 150.0f, -1.0f);
    FrameWheelLock(500.0f, 150.0f, 0.0f);
}

// cmd_offsets: two commands in one list, so each carries cumulative vertex/index offsets.

static void DumpCmdOffsets()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario cmd_offsets\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    ImGui::NewFrame();
    ImDrawList list(ImGui::GetDrawListSharedData());
    list._ResetForNewFrame();
    list._SetPixelDensity(1.0f);
    list.Flags = 0;
    list.AddRectFilled(ImVec2(10.0f, 10.0f), ImVec2(50.0f, 30.0f), IM_COL32(255, 0, 0, 255));
    list.PushClipRect(ImVec2(20.0f, 20.0f), ImVec2(80.0f, 60.0f), false);
    list.AddRectFilled(ImVec2(30.0f, 25.0f), ImVec2(70.0f, 45.0f), IM_COL32(0, 255, 0, 255));
    DumpDrawListBody(list);
    ImGui::EndFrame();
}

// scene_window_button: a titled window with a button inside, compared as one draw list.
//
// A window does not know its title bar height during its first frame, so its content is laid out
// as if there were no band and moves down afterwards. The scene settles for a fixed number of
// frames and then compares one, printing the settled title height so a mismatch can be explained.
static void FrameSceneWindowButton(bool dump, ImDrawList** out_list, float* title_height)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("W", NULL, flags);
    if (title_height != NULL)
        *title_height = ImGui::GetCurrentWindow()->TitleBarHeight;
    ImGui::Button("Hi");
    if (dump && out_list != NULL)
        *out_list = ImGui::GetWindowDrawList();
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpSceneWindowButtonScenario()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario scene_window_button\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 6);

    const int settle_frames = 12;
    for (int i = 0; i < settle_frames; i++)
        FrameSceneWindowButton(false, NULL, NULL);
    ImDrawList* dl = NULL;
    float title_height = 0.0f;
    FrameSceneWindowButton(true, &dl, &title_height);
    fprintf(stderr, "settle %d title_height %08x\n", settle_frames, Bits(title_height));
    if (dl != NULL)
        DumpDrawListBody(*dl);
}

// button_states: the same button resting, hovered and held down. Each state is one dumped section,
// and the frames run in order so the press registers as it would in a real session.
static void FrameButtonState(float mx, float my, bool down, ImDrawList** out_list)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
    ImGui::Button("Hi");
    if (out_list != NULL)
        *out_list = ImGui::GetWindowDrawList();
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpButtonStates()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario button_states\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 6);

    // Each state must be printed right after its frame: the reference reuses the window's draw
    // list, so holding a pointer would show the last frame three times.
    ImDrawList* list = NULL;
    FrameButtonState(0.0f, 0.0f, false, &list);
    printf("state resting\n");
    DumpDrawListBody(*list);

    FrameButtonState(85.0f, 85.0f, false, &list);
    printf("state hovered\n");
    DumpDrawListBody(*list);

    FrameButtonState(85.0f, 85.0f, true, NULL);
    FrameButtonState(85.0f, 85.0f, true, &list);
    printf("state active\n");
    DumpDrawListBody(*list);
}

// button_border: the same button with FrameBorderSize set, so the frame has an outline.
static void DumpButtonBorder()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario button_border\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'W', 'H', 'i', 'e', 'l', 'o' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 6);

    ImGui::GetStyle().FrameBorderSize = 1.0f;
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    ImGui::SetCursorScreenPos(ImVec2(80.0f, 80.0f));
    ImGui::Button("Hi");
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImGui::End();
    DumpDrawListBody(*dl);
    ImGui::EndFrame();
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
    if (argc > 2)
        g_fixture_path = argv[2];

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

    if (strcmp(name, "scene_window_button") == 0)
    {
        DumpSceneWindowButtonScenario();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "button_border") == 0)
    {
        DumpButtonBorder();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "button_states") == 0)
    {
        DumpButtonStates();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_icons") == 0)
    {
        DumpWindowIcons();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_wheel_lock") == 0)
    {
        DumpWindowWheelLock();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_autosize_change") == 0)
    {
        DumpWindowAutoSizeChange();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_autosize") == 0)
    {
        DumpWindowAutoSize();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_wheel") == 0)
    {
        DumpWindowWheel();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_scrollbar_drag") == 0)
    {
        DumpWindowScrollbarDrag();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_scrollbar") == 0)
    {
        DumpWindowScrollbar();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_scroll") == 0)
    {
        DumpWindowScroll();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_move") == 0)
    {
        DumpWindowsMove();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_order") == 0)
    {
        DumpWindowsOrder();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_icons_drag") == 0)
    {
        DumpWindowIconsDrag();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_icons_states") == 0)
    {
        DumpWindowIconsStates();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "aa_strokes") == 0)
    {
        DumpAaStrokes();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "cmd_offsets") == 0)
    {
        DumpCmdOffsets();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_title") == 0)
    {
        DumpWindowTitle();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_border") == 0)
    {
        DumpWindowBorder();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_bg") == 0)
    {
        DumpWindowBg();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "button_draw") == 0)
    {
        DumpButtonDraw();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "text_draw") == 0)
    {        DumpTextDraw();
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
    else if (strcmp(name, "aa_rect") == 0) scene = SceneAaRect;
    else if (strcmp(name, "aa_line") == 0) scene = SceneAaLine;
    else if (strcmp(name, "aa_bevel") == 0) scene = SceneAaBevel;
    else if (strcmp(name, "aa_polyline") == 0) scene = SceneAaPolyline;
    else if (strcmp(name, "aa_border") == 0) scene = SceneAaBorder;
    if (strcmp(name, "clip_stack") == 0)
    {
        ImGui::NewFrame();
        DumpClipStack();
        ImGui::EndFrame();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "aa_round_rect") == 0)
    {
        DumpAaRoundRect();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "arc_steps") == 0)
    {
        DumpArcSteps();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "bezier_points") == 0)
    {
        DumpBezierPoints();
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