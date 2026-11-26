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

static ImVec2 PSub(const ImVec2& a, const ImVec2& b)
{
    return ImVec2(a.x - b.x, a.y - b.y);
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

// io_text_events: the character queue, and the press/release/repeat timeline of a held key.
static void DumpIoText()
{
    struct KeyEv
    {
        ImGuiKey key;
        bool down;
    };
    struct TextFrame
    {
        int count;
        KeyEv ev[2];
        const char* chars;
    };
    const KeyEv none = { ImGuiKey_None, false };
    const KeyEv la_down = { ImGuiKey_LeftArrow, true };
    const KeyEv la_up = { ImGuiKey_LeftArrow, false };
    // 0..3: characters, including a two-byte and a four-byte code point. 4..14: a key held for
    // eleven frames at 0.05 s and then released, which crosses io.KeyRepeatDelay (0.275). 15..22:
    // the modifier keys, and a sample of the wider key enum.
    const TextFrame frames[23] = {
        { 0, { none, none }, NULL },
        { 0, { none, none }, "ab" },
        { 0, { none, none }, NULL },
        { 0, { none, none }, "\xC3\xA9\xF0\x9F\x98\x80" },
        { 1, { la_down, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 0, { none, none }, NULL },
        { 1, { la_up, none }, NULL },
        { 0, { none, none }, NULL },
        { 2, { { (ImGuiKey)ImGuiMod_Ctrl, true }, { ImGuiKey_Tab, true } }, NULL },
        { 2, { { (ImGuiKey)ImGuiMod_Ctrl, false }, { ImGuiKey_Tab, false } }, NULL },
        { 1, { { ImGuiKey_LeftCtrl, true }, none }, NULL },
        { 1, { { ImGuiKey_LeftCtrl, false }, none }, NULL },
        { 1, { { ImGuiKey_F1, true }, none }, NULL },
        { 1, { { ImGuiKey_KeypadEnter, true }, none }, NULL },
        { 1, { { ImGuiKey_Enter, true }, none }, NULL },
        { 1, { { ImGuiKey_Escape, true }, none }, NULL },
    };

    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario io_text_events\n");

    ImGuiIO& io = ImGui::GetIO();
    for (int i = 0; i < 23; i++)
    {
        io.DeltaTime = 0.05f;
        for (int e = 0; e < frames[i].count; e++)
            io.AddKeyEvent(frames[i].ev[e].key, frames[i].ev[e].down);
        if (frames[i].chars != NULL)
            io.AddInputCharactersUTF8(frames[i].chars);
        ImGui::NewFrame();

        const ImGuiKeyData* la = ImGui::GetKeyData(ImGuiKey_LeftArrow);
        printf("f%d t %08x chars %d la_down %d la_press %d la_rep %d la_rel %d la_dur %08x la_prev %08x "
               "mods %d%d%d%d down %d%d%d%d%d%d\n",
            i, Bits(ImGui::GetTime()), io.InputQueueCharacters.Size,
            la->Down ? 1 : 0, ImGui::IsKeyPressed(ImGuiKey_LeftArrow, false) ? 1 : 0,
            ImGui::IsKeyPressed(ImGuiKey_LeftArrow, true) ? 1 : 0,
            ImGui::IsKeyReleased(ImGuiKey_LeftArrow) ? 1 : 0,
            Bits(la->DownDuration), Bits(la->DownDurationPrev),
            io.KeyCtrl ? 1 : 0, io.KeyShift ? 1 : 0, io.KeyAlt ? 1 : 0, io.KeySuper ? 1 : 0,
            ImGui::IsKeyDown(ImGuiKey_Tab) ? 1 : 0, ImGui::IsKeyDown(ImGuiKey_F1) ? 1 : 0,
            ImGui::IsKeyDown(ImGuiKey_KeypadEnter) ? 1 : 0, ImGui::IsKeyDown(ImGuiKey_Enter) ? 1 : 0,
            ImGui::IsKeyDown(ImGuiKey_Escape) ? 1 : 0, ImGui::IsKeyDown(ImGuiKey_LeftCtrl) ? 1 : 0);
        for (int c = 0; c < io.InputQueueCharacters.Size; c++)
            printf("c %04x\n", (unsigned int)io.InputQueueCharacters[c]);
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

// cmd_split: one window draw list where every clip change has to cut the command the same way the
// reference does. A clip that does not change must not start a command; one that does must; and a
// push/pop pair that draws nothing must merge back into the command it came from.
static void DumpCmdSplit()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario cmd_split\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    // The clip the window started with, which is the clip of the command already in the buffer. A
    // loose ImDrawList is avoided on purpose: outside a window it faults in this reference build.
    const ImRect win = ImGui::GetCurrentWindow()->ClipRect;
    const ImVec2 cmin = win.Min;
    const ImVec2 cmax = win.Max;

    // 1. The window clip first, the command every later step is compared against.
    dl->AddRectFilled(ImVec2(70.0f, 70.0f), ImVec2(110.0f, 90.0f), COL);
    // 2. Pushing that very rectangle is not a change: no new command.
    dl->PushClipRect(cmin, cmax, false);
    dl->AddRectFilled(ImVec2(70.0f, 100.0f), ImVec2(110.0f, 120.0f), COL);
    // 3. A second identical push, nested, neither.
    dl->PushClipRect(cmin, cmax, false);
    dl->AddRectFilled(ImVec2(70.0f, 130.0f), ImVec2(110.0f, 150.0f), COL);
    dl->PopClipRect();
    // 4. Popping back to the clip the command already carries: still no split.
    dl->PopClipRect();
    dl->AddRectFilled(ImVec2(120.0f, 70.0f), ImVec2(160.0f, 90.0f), COL);
    // 5. Intersecting with a larger rectangle leaves the clip alone: no split.
    dl->PushClipRect(ImVec2(cmin.x - 20.0f, cmin.y - 20.0f),
        ImVec2(cmax.x + 20.0f, cmax.y + 20.0f), true);
    dl->AddRectFilled(ImVec2(120.0f, 100.0f), ImVec2(160.0f, 120.0f), COL);
    dl->PopClipRect();
    // 6. Intersecting with a smaller one does change it: the command is cut here.
    dl->PushClipRect(ImVec2(130.0f, 60.0f), ImVec2(200.0f, 200.0f), true);
    dl->AddRectFilled(ImVec2(120.0f, 130.0f), ImVec2(180.0f, 160.0f), COL);
    dl->PopClipRect();
    // 7. Back at the window clip, which the command just closed does not carry: cut again.
    dl->AddRectFilled(ImVec2(120.0f, 170.0f), ImVec2(160.0f, 190.0f), COL);
    // 8. A push that changes the clip and a pop that undoes it with nothing drawn in between: the
    // empty command merges back into the one it came from, so this rectangle joins the previous one
    // instead of opening a fourth command.
    dl->PushClipRect(ImVec2(62.0f, 62.0f), ImVec2(300.0f, 150.0f), false);
    dl->PopClipRect();
    dl->AddRectFilled(ImVec2(70.0f, 170.0f), ImVec2(110.0f, 190.0f), COL);

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// aa_circles: filled discs and stroked circles below the fast arc cutoff radius (140.11), the range
// that samples the pinned arc table instead of calling cos/sin. Every automatic segment step the
// table can ask for is covered, plus the alpha-multiplied minimum radius and the collapsed stroke.
static void WriteLineUvFixture();
static void WriteWindowIconsFixture(ImFont* font, const char* bake_order, int bake_count);

static void DumpAaCircles()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario aa_circles\n");

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

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(40.0f, 40.0f));
    ImGui::SetNextWindowSize(ImVec2(520.0f, 400.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    ImDrawList* dl = ImGui::GetWindowDrawList();

    // Filled discs: radii 2, 5, 12, 20, 30, 45 and 63 take the table steps 12, 4, 3, 2, 2, 1 and 1.
    dl->AddCircleFilled(ImVec2(100.0f, 100.0f), 2.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(140.0f, 100.0f), 5.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(190.0f, 100.0f), 12.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(250.0f, 100.0f), 20.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(320.0f, 100.0f), 30.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(400.0f, 100.0f), 45.0f, COL, 0);
    dl->AddCircleFilled(ImVec2(490.0f, 100.0f), 63.0f, COL, 0);
    // Below the minimum radius the reference scales the alpha and draws at the minimum instead.
    dl->AddCircleFilled(ImVec2(100.0f, 230.0f), 0.6f, COL, 0);

    // Stroked circles, inside stroke by default.
    dl->AddCircle(ImVec2(140.0f, 230.0f), 5.0f, COL, 0, 1.0f);
    dl->AddCircle(ImVec2(190.0f, 230.0f), 10.0f, COL, 0, 1.0f);
    dl->AddCircle(ImVec2(250.0f, 230.0f), 20.0f, COL, 0, 3.0f);
    dl->AddCircle(ImVec2(320.0f, 230.0f), 30.0f, COL, 0, 1.0f);
    dl->AddCircle(ImVec2(430.0f, 230.0f), 60.0f, COL, 0, 2.0f);
    // A stroke thicker than the radius collapses into a filled disc of the outer radius.
    dl->AddCircle(ImVec2(490.0f, 330.0f), 2.0f, COL, 0, 3.0f);

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_basic: a table with three fixed columns and two rows of text, the first cut of the table
// layout: no borders, no headers, no sizing policy and no table scrolling.
static void WriteGlyphFixture(ImFont* font, const char* bake_order, int bake_count);

static void DumpTablesBasic()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_basic\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', '0', '1', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow();
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_headers: the same three fixed columns, but with TableHeadersRow first: the header row is
// 17 px tall (13 + 2*CellPadding.y), carries the column names and paints its own background, and it
// counts for TableGetRowIndex().
static void DumpTablesHeaders()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_headers\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'A', 'B', 'C', '0', '1', '2' };
    for (int i = 0; i < 6; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 6);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    ImGui::TableHeadersRow();
    printf("after headers rowindex %d\n", ImGui::TableGetRowIndex());

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow();
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x rowindex %d\n", Bits(end_cursor.x), Bits(end_cursor.y),
        ImGui::TableGetRowIndex());

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_rowbg: the same fixed columns with ImGuiTableFlags_RowBg, so every other data row gets the
// TableRowBgAlt band. The header row keeps its own band and does not advance the odd/even counter.
static void DumpTablesRowBg()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_rowbg\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[7] = { 'A', 'B', 'C', '0', '1', '2', '3' };
    for (int i = 0; i < 7; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 7);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_RowBg, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    ImGui::TableHeadersRow();
    printf("after headers rowindex %d\n", ImGui::TableGetRowIndex());

    for (int row = 0; row < 4; row++)
    {
        ImGui::TableNextRow();
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_cellbg: `TableSetBgColor(ImGuiTableBgTarget_CellBg, ...)` on two cells of the first row. The
// fills are collected while the cells are submitted and painted when the row closes, into the
// background channel, each clipped to its own column.
static void DumpTablesCellBg()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_cellbg\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', '0', '1', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow();
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            if (row == 0 && col == 0)
                ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, IM_COL32(200, 40, 40, 255));
            if (row == 0 && col == 2)
                ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, IM_COL32(40, 200, 40, 255));
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_borders: `ImGuiTableFlags_Borders` (inner and outer, both axes). The borders are drawn by
// TableEndRow (the line above each row after the first) and by TableDrawBorders when the table
// closes (the outer rectangle and one vertical line per column), all into the background channel.
static void DumpTablesBorders()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_borders\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'A', 'B', 'C', '0', '1', '2' };
    for (int i = 0; i < 6; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 6);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_BordersInner, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    ImGui::TableHeadersRow();
    printf("after headers rowindex %d\n", ImGui::TableGetRowIndex());

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow();
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_borders_outer: `ImGuiTableFlags_Borders`, the outer rectangle included. Cells hold a
// `Dummy` instead of text on purpose: with whole coordinates the outer border takes the
// pixel-aligned fast path of `AddRect` (white texture), so no atlas fixture is needed at all.
static void DumpTablesBordersOuter()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_borders_outer\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_Borders, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    for (int row = 0; row < 4; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            ImGui::Dummy(ImVec2(10.0f, 10.0f));
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_stretch: `ImGuiTableFlags_SizingStretchSame` with columns that ask for no width at all, so
// the three columns share the table's width in equal parts and the rounding remainder goes to the
// right-most one.
static void DumpTablesStretch()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_stretch\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', '0', '1', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_SizingStretchSame,
        ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A");
    ImGui::TableSetupColumn("B");
    ImGui::TableSetupColumn("C");

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_stretch_weights: one fixed column and two stretch columns with explicit weights (2 and 1),
// so the leftover width after the fixed one is shared in that ratio.
static void DumpTablesStretchWeights()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_stretch_weights\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', '0', '1', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_SizingStretchProp,
        ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthStretch, 2.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthStretch, 1.0f);

    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_set_column: `TableSetColumnIndex` addresses cells out of order in one row. The row still
// grows to the tallest cell (a 30 px `Dummy` in the middle column), whatever order they come in.
static void DumpTablesSetColumn()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_set_column\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', '0', '2' };
    for (int i = 0; i < 3; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 3);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    ImGui::TableNextRow();
    const int order[3] = { 1, 0, 2 };
    for (int i = 0; i < 3; i++)
    {
        const int col = order[i];
        const bool visible = ImGui::TableSetColumnIndex(col);
        const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
        printf("cell %d visible %d cursor %08x %08x rowindex %d colindex %d\n", col,
            (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
            ImGui::TableGetColumnIndex());
        if (col == 1)
        {
            ImGui::Dummy(ImVec2(10.0f, 30.0f));
        }
        else
        {
            ImGui::Text(col == 0 ? "A0" : "A2");
        }
        const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
        const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
        printf("cell %d item %08x %08x %08x %08x\n", col, Bits(imin.x), Bits(imin.y),
            Bits(imax.x), Bits(imax.y));
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// tables_names: the table's public column API — `TableGetColumnCount`, `TableGetColumnName` and
// `TableGetColumnIndex` — inside the table and after it closes.
static void DumpTablesNames()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_names\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    printf("count %d\n", ImGui::TableGetColumnCount());
    printf("name before declaration %d '%s'\n", 0, ImGui::TableGetColumnName(0));
    ImGui::TableSetupColumn("Alpha", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("Beta", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("Gamma", ImGuiTableColumnFlags_WidthFixed, 100.0f);
    for (int n = 0; n < ImGui::TableGetColumnCount(); n++)
        printf("name %d '%s'\n", n, ImGui::TableGetColumnName(n));

    ImGui::TableNextRow();
    for (int n = 0; n < 3; n++)
    {
        ImGui::TableSetColumnIndex(n);
        printf("inside name %d '%s' colindex %d\n", n, ImGui::TableGetColumnName(n),
            ImGui::TableGetColumnIndex());
    }
    ImGui::EndTable();
    printf("after count %d name0 '%s'\n", ImGui::TableGetColumnCount(),
        ImGui::TableGetColumnName(0));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_header_id: a column label may carry a "##id" suffix. `TableGetColumnName` keeps the whole
// string, while the header draws only the part before "##" (FindRenderedTextEnd).
static void DumpTablesHeaderId()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_header_id\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[8] = { 'A', 'l', 'p', 'h', 'a', 'B', 'e', 't' };
    for (int i = 0; i < 8; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 8);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("Alpha##1", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("Beta##2", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    printf("name 0 '%s' name 1 '%s'\n", ImGui::TableGetColumnName(0), ImGui::TableGetColumnName(1));

    ImGui::TableHeadersRow();
    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_clip: a 10 px column whose text is wider than the column. The reference clips the items of
// a cell to the column (`window->ClipRect`) and, because this column's content overflows its clip,
// `TableMergeDrawChannels` keeps it in a command of its own carrying the column's clip.
static void DumpTablesClip()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_clip\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', '0' };
    for (int i = 0; i < 2; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 2);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 10.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    for (int col = 0; col < 2; col++)
    {
        ImGui::TableNextColumn();
        const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
        printf("cell %d cursor %08x %08x\n", col, Bits(cursor.x), Bits(cursor.y));
        ImGui::Text("A00");
        const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
        const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
        printf("cell %d item %08x %08x %08x %08x\n", col, Bits(imin.x), Bits(imin.y),
            Bits(imax.x), Bits(imax.y));
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_auto_width: columns declared with no width at all, in a table that is not resizable. The
// first frame gives them the minimum width and the contents of that frame size them for the next,
// so the widths settle on the second frame and the third confirms it.
static void DumpTablesAutoWidth()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_auto_width\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', 'B', 'C', '0' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 3; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 window_pos = ImGui::GetWindowPos();

        const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_SizingFixedFit,
            ImVec2(0.0f, 0.0f), 0.0f);
        if (frame == 0)
            printf("table begin %d\n", (int)began);
        ImGui::TableSetupColumn("A");
        ImGui::TableSetupColumn("B");
        ImGui::TableSetupColumn("C");

        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        const char* labels[3] = { "A0", "B0", "C0" };
        for (int col = 0; col < 3; col++)
        {
            ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("frame %d cell %d cursor %08x %08x\n", frame, col, Bits(cursor.x),
                Bits(cursor.y));
            ImGui::Text("%s", labels[col]);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("frame %d cell %d item %08x %08x %08x %08x\n", frame, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }

        ImGui::EndTable();
        const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
        if (frame == 2)
        {
            printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_header_inrow: `TableHeader` outside a header row. The reference fills that cell with
// `TableHeaderBg` (the else branch of TableHeader), which is the only background a plain row gets.
static void DumpTablesHeaderInRow()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_header_inrow\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (int i = 0; i < 2; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 2);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);

    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableSetColumnIndex(0);
    ImGui::TableHeader("A");
    {
        const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
        const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
        printf("header item %08x %08x %08x %08x\n", Bits(imin.x), Bits(imin.y), Bits(imax.x),
            Bits(imax.y));
    }
    ImGui::TableSetColumnIndex(1);
    {
        const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
        printf("cell 1 cursor %08x %08x\n", Bits(cursor.x), Bits(cursor.y));
        ImGui::Text("B");
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_column_flags: `TableGetColumnFlags` after the table policy has filled in the width kind,
// the resize bit and the indentation bit, for a fixed table and for a stretch one.
static void DumpTablesColumnFlags()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_column_flags\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_SizingFixedFit,
        ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d count %d\n", (int)began, ImGui::TableGetColumnCount());
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B");
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_NoResize, 80.0f);
    for (int n = 0; n < 3; n++)
        printf("fixed column %d flags %08x\n", n, (unsigned)ImGui::TableGetColumnFlags(n));
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    for (int n = 0; n < 3; n++)
        printf("fixed inside %d flags %08x\n", n, (unsigned)ImGui::TableGetColumnFlags(n));
    ImGui::EndTable();
    printf("after flags %08x\n", (unsigned)ImGui::TableGetColumnFlags(0));

    const bool began2 = ImGui::BeginTable("T2", 2, ImGuiTableFlags_SizingStretchSame,
        ImVec2(0.0f, 0.0f), 0.0f);
    printf("table2 begin %d\n", (int)began2);
    ImGui::TableSetupColumn("A");
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 40.0f);
    for (int n = 0; n < 2; n++)
        printf("stretch column %d flags %08x\n", n, (unsigned)ImGui::TableGetColumnFlags(n));
    ImGui::EndTable();

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_rowbg_headers: `ImGuiTableFlags_RowBg` with a header row first, so the bands of the data rows
// start where the header leaves the counter.
static void DumpTablesRowBgHeaders()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_rowbg_headers\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (int i = 0; i < 2; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 2);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 1, ImGuiTableFlags_RowBg, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableHeadersRow();
    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        ImGui::TableNextColumn();
        ImGui::Dummy(ImVec2(10.0f, 10.0f));
    }

    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_next_column: `TableNextColumn` walks the row and, past the last column, starts a new row by
// itself. Five calls on a two-column table therefore fill two rows and open a third.
static void DumpTablesNextColumn()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_next_column\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);

    for (int n = 0; n < 5; n++)
    {
        const bool visible = ImGui::TableNextColumn();
        const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
        printf("cell %d visible %d row %d col %d cursor %08x %08x\n", n, (int)visible,
            ImGui::TableGetRowIndex(), ImGui::TableGetColumnIndex(), Bits(cursor.x),
            Bits(cursor.y));
        ImGui::Dummy(ImVec2(10.0f, 10.0f));
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_header_hover: the mouse rests on the second header cell. Headers use `AllowOverlap`, which
// only reports hover when the item was already hovered on the previous frame, so the highlight shows
// up on the third frame; the dump is the third one.
static void DumpTablesHeaderHover()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_header_hover\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (int i = 0; i < 3; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 3; frame++)
    {
        io.MousePos = ImVec2(100.0f, 75.0f);
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("T", 1, 0, ImVec2(0.0f, 100.0f), 0.0f);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableHeadersRow();
        ImGui::EndTable();
        if (frame == 2)
        {
            printf("table begin %d\n", (int)began);
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_header_active: the mouse presses the header of a one-column table after hovering it long
// enough, so the cell takes `ImGuiCol_HeaderActive` for as long as the button stays down.
static void DumpTablesHeaderActive()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_header_active\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[1] = { 'A' };
    baked->FindGlyph((ImWchar)bake_order[0]);
    WriteGlyphFixture(font, bake_order, 1);

    io.DeltaTime = 0.1f;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 4; frame++)
    {
        io.MousePos = ImVec2(100.0f, 75.0f);
        io.MouseDown[0] = (frame == 3);
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("T", 1, 0, ImVec2(0.0f, 100.0f), 0.0f);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableHeadersRow();
        ImGui::EndTable();
        if (frame == 3)
        {
            printf("table begin %d\n", (int)began);
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_hovered: `TableGetHoveredColumn` and `TableGetHoveredRow` with the mouse over the second
// column. The column is immediate, the row has one frame of latency.
static void DumpTablesHovered()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_hovered\n");

    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 4; frame++)
    {
        io.MousePos = ImVec2(150.0f, 95.0f);
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
        if (frame == 0)
            printf("table begin %d\n", (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        for (int row = 0; row < 2; row++)
        {
            ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
            for (int col = 0; col < 2; col++)
            {
                ImGui::TableNextColumn();
                ImGui::Dummy(ImVec2(10.0f, 10.0f));
            }
        }
        printf("frame %d hovered col %d row %d\n", frame, ImGui::TableGetHoveredColumn(),
            ImGui::TableGetHoveredRow());
        ImGui::EndTable();
        if (frame == 3)
            DumpDrawListBody(*dl);
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_stretch_prop: `SizingStretchProp` derives each stretch column's weight from the width its
// contents reached, so the columns end up proportional to their labels after the first frame.
static void DumpTablesStretchProp()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_stretch_prop\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (int i = 0; i < 3; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 3; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 window_pos = ImGui::GetWindowPos();

        const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_SizingStretchProp,
            ImVec2(0.0f, 0.0f), 0.0f);
        if (frame == 0)
            printf("table begin %d\n", (int)began);
        ImGui::TableSetupColumn("A");
        ImGui::TableSetupColumn("B");
        ImGui::TableSetupColumn("C");
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        const char* labels[3] = { "A", "BB", "CCC" };
        for (int col = 0; col < 3; col++)
        {
            ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("frame %d cell %d cursor %08x %08x\n", frame, col, Bits(cursor.x),
                Bits(cursor.y));
            ImGui::Text("%s", labels[col]);
        }
        ImGui::EndTable();
        if (frame == 2)
        {
            const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_row_bg_color: `TableSetBgColor(RowBg0, ...)` paints one row with an explicit colour, with
// no need for the `RowBg` flag; the next row goes back to having no background.
static void DumpTablesRowBgColor()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_row_bg_color\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 1, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    for (int row = 0; row < 2; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        if (row == 0)
            ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, 0xFF336699u);
        ImGui::TableNextColumn();
        ImGui::Dummy(ImVec2(10.0f, 10.0f));
    }
    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_column_bounds: `TableSetColumnIndex` past the last column returns false and leaves the
// cell cursor alone, and `TableNextColumn` called outside a row starts one.
static void DumpTablesColumnBounds()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_column_bounds\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);

    const bool first = ImGui::TableNextColumn();
    const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("next column outside a row %d row %d col %d cursor %08x %08x\n", (int)first,
        ImGui::TableGetRowIndex(), ImGui::TableGetColumnIndex(), Bits(cursor.x), Bits(cursor.y));

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_cell_bg_twice: two `TableSetBgColor(CellBg)` calls on the same cell. The second one replaces
// the first (the reference only appends when the column changes), while the next cell keeps its own.
static void DumpTablesCellBgTwice()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_cell_bg_twice\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, 0xFFFF0000u);
    ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, 0xFF00FF00u);
    ImGui::Dummy(ImVec2(10.0f, 10.0f));
    ImGui::TableNextColumn();
    ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, 0xFF0000FFu);
    ImGui::Dummy(ImVec2(10.0f, 10.0f));
    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_column_hovered_flag: the `IsHovered` bit of `TableGetColumnFlags` marks the column under the
// mouse, as `TableGetHoveredColumn` reports it.
static void DumpTablesColumnHoveredFlag()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_column_hovered_flag\n");

    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 3; frame++)
    {
        io.MousePos = ImVec2(150.0f, 95.0f);
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
        if (frame == 0)
            printf("table begin %d\n", (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        for (int row = 0; row < 2; row++)
        {
            ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
            for (int col = 0; col < 2; col++)
            {
                ImGui::TableNextColumn();
                ImGui::Dummy(ImVec2(10.0f, 10.0f));
            }
        }
        printf("frame %d flags %08x %08x\n", frame, (unsigned)ImGui::TableGetColumnFlags(0),
            (unsigned)ImGui::TableGetColumnFlags(1));
        ImGui::EndTable();
        if (frame == 2)
            DumpDrawListBody(*dl);
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_scroll_child: a table with `ScrollY` builds its rows in an inner child window (no padding,
// no border) with a draw list of its own, so the frame's draw data carries two lists: the host's
// empty one and the child's with the table. The content fits, so no scrollbar shows yet.
static void DumpTablesScrollChild()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_scroll_child\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[6] = { 'A', 'B', 'C', 'R', '0', '1' };
    for (int i = 0; i < 6; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 6);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        const bool began = ImGui::BeginTable("SCROLL", 3,
            ImGuiTableFlags_ScrollY | ImGuiTableFlags_RowBg, ImVec2(0.0f, 400.0f), 0.0f);
        printf("frame %d table begin %d\n", frame, (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableHeadersRow();
        for (int row = 0; row < 2; row++)
        {
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            char text[8];
            snprintf(text, sizeof(text), "R%d", row);
            ImGui::Text("%s", text);
            ImGui::TableNextColumn();
            ImGui::Dummy(ImVec2(10.0f, 10.0f));
            ImGui::TableNextColumn();
            ImGui::Dummy(ImVec2(10.0f, 10.0f));
        }
        ImGui::EndTable();
        ImGui::End();
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", frame, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// tables_sort_multi: a Shift+click adds a second sorting column when the table is `SortMulti`. First
// a plain click on A turns its default ascending sort into descending, then a Shift+click on B
// appends it as the second spec, which makes B draw its order number "2".
static void DumpTablesSortMulti()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_sort_multi\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', 'B', 'C', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 4);

    io.DeltaTime = 0.1f;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    static const float xs[10] = { 0.0f, 100.0f, 100.0f, 100.0f, 100.0f, 180.0f, 180.0f, 180.0f,
        180.0f, 0.0f };
    static const float ys[10] = { 0.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f,
        0.0f };
    static const bool downs[10] = { false, false, true, false, false, false, true, false, false,
        false };
    for (int frame = 0; frame < 10; frame++)
    {
        io.AddKeyEvent(ImGuiMod_Shift, frame == 7);
        io.MousePos = ImVec2(xs[frame], ys[frame]);
        io.MouseDown[0] = downs[frame];
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("SORTM", 3,
            ImGuiTableFlags_Sortable | ImGuiTableFlags_SortMulti, ImVec2(0.0f, 0.0f), 0.0f);
        printf("frame %d table begin %d\n", frame, (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableHeadersRow();
        const ImGuiTableSortSpecs* specs = ImGui::TableGetSortSpecs();
        printf("frame %d hovered %d\n", frame, ImGui::TableGetHoveredColumn());
        printf("frame %d specs %d count %d dirty %d shift %d", frame, specs != NULL,
            specs ? specs->SpecsCount : -1, specs ? (int)specs->SpecsDirty : -1,
            (int)io.KeyShift);
        if (specs != NULL)
            for (int n = 0; n < specs->SpecsCount; n++)
                printf(" %d:%d", specs->Specs[n].ColumnIndex, (int)specs->Specs[n].SortDirection);
        printf("\n");
        ImGui::EndTable();
        if (frame == 0 || frame == 4 || frame == 7 || frame == 8 || frame == 9)
        {
            printf("frame %d dump\n", frame);
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_reorder: dragging a header past its edge moves the column to the neighbour's display
// position. The move is queued by the drag and applied by the next BeginTable, so the frame the drag
// starts on still draws the old order. Two moves: B before A, then C before A. The hovered column at
// a fixed mouse position is the text proof that the display order changed.
static void DumpTablesReorder()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_reorder\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (int i = 0; i < 3; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    static const float xs[17] = { 0.0f, 180.0f, 180.0f, 180.0f, 120.0f, 120.0f, 120.0f, 120.0f,
        280.0f, 280.0f, 280.0f, 170.0f, 170.0f, 170.0f, 170.0f, 170.0f, 0.0f };
    static const float ys[17] = { 0.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f,
        75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 0.0f };
    static const bool downs[17] = { false, false, true, true, true, true, false, false, false, true,
        true, true, true, false, false, false, false };
    for (int frame = 0; frame < 17; frame++)
    {
        io.MousePos = ImVec2(xs[frame], ys[frame]);
        io.MouseDown[0] = downs[frame];
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("ORDER", 3, ImGuiTableFlags_Reorderable,
            ImVec2(0.0f, 0.0f), 0.0f);
        printf("frame %d table begin %d\n", frame, (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);
        ImGui::TableHeadersRow();
        printf("frame %d hovered %d\n", frame, ImGui::TableGetHoveredColumn());
        ImGui::EndTable();
        if (frame == 0 || frame == 4 || frame == 5 || frame == 12 || frame == 13 || frame == 15)
        {
            printf("frame %d dump\n", frame);
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_sort_click: clicking a header cycles its sort direction. The sequence covers the hover
// latch the header needs before it presses, the release that applies the click (the arrow keeps the
// old direction on that frame) and the frame after it, twice: ascending -> descending -> ascending.
// The specs are handed out right after the header row, so their dirty flag shows the click too.
static void DumpTablesSortClick()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_sort_click\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (int i = 0; i < 3; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    static const float xs[12] = { 0.0f, 100.0f, 100.0f, 100.0f, 100.0f, 100.0f, 100.0f, 100.0f,
        100.0f, 100.0f, 0.0f, 0.0f };
    static const float ys[12] = { 0.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f, 75.0f,
        75.0f, 0.0f, 0.0f };
    static const bool downs[12] = { false, false, true, true, false, false, true, true, false,
        false, false, false };
    for (int frame = 0; frame < 12; frame++)
    {
        io.MousePos = ImVec2(xs[frame], ys[frame]);
        io.MouseDown[0] = downs[frame];
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();

        const bool began = ImGui::BeginTable("SORT", 3, ImGuiTableFlags_Sortable,
            ImVec2(0.0f, 0.0f), 0.0f);
        printf("frame %d table begin %d\n", frame, (int)began);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort, 60.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
        ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_NoSort, 100.0f);
        ImGui::TableHeadersRow();
        const ImGuiTableSortSpecs* specs = ImGui::TableGetSortSpecs();
        printf("frame %d specs %d count %d dirty %d", frame, specs != NULL,
            specs ? specs->SpecsCount : -1, specs ? (int)specs->SpecsDirty : -1);
        if (specs != NULL)
            for (int n = 0; n < specs->SpecsCount; n++)
                printf(" %d:%d", specs->Specs[n].ColumnIndex, (int)specs->Specs[n].SortDirection);
        printf("\n");
        ImGui::EndTable();
        if (frame == 0 || frame == 2 || frame == 4 || frame == 5 || frame == 8 || frame == 9)
        {
            printf("frame %d dump\n", frame);
            DumpDrawListBody(*dl);
        }
        ImGui::End();
        ImGui::EndFrame();
    }
}
// tables_sort_arrow: the sort arrow a sortable header draws for the columns the specs point at.
// `SortMulti` lets two columns keep sort orders 0 and 1, so the second one also draws its order
// number, and `PreferSortDescending` makes its arrow point down. The third column is `NoSort`, so it
// draws no arrow at all. `TableGetSortSpecs` runs before the header row because it is what linearizes
// the two orders the `DefaultSort` columns start with.
static void DumpTablesSortArrow()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_sort_arrow\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', 'B', 'C', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    // A throwaway frame wires the atlas line textures: the arrows are strokes that sample them.
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 4);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("S", NULL, flags);
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("SORT", 3,
        ImGuiTableFlags_Sortable | ImGuiTableFlags_SortMulti, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort |
        ImGuiTableColumnFlags_PreferSortDescending, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_NoSort, 100.0f);

    const ImGuiTableSortSpecs* specs = ImGui::TableGetSortSpecs();
    printf("specs %d count %d dirty %d\n", specs != NULL, specs ? specs->SpecsCount : -1,
        specs ? (int)specs->SpecsDirty : -1);
    if (specs != NULL)
        for (int n = 0; n < specs->SpecsCount; n++)
            printf("spec %d column %d direction %d\n", n, specs->Specs[n].ColumnIndex,
                (int)specs->Specs[n].SortDirection);

    ImGui::TableHeadersRow();
    printf("after headers rowindex %d\n", ImGui::TableGetRowIndex());

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), ImGui::GetWindowPos());
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_sort_specs: `TableGetSortSpecs` right after the first row, on a sortable table whose second
// column asked for the default sort.
static void DumpTablesSortSpecs()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_sort_specs\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_Sortable, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_NoSort, 100.0f);
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    ImGui::Dummy(ImVec2(10.0f, 10.0f));

    const ImGuiTableSortSpecs* specs = ImGui::TableGetSortSpecs();
    printf("specs %d count %d dirty %d\n", specs != NULL, specs ? specs->SpecsCount : -1,
        specs ? (int)specs->SpecsDirty : -1);
    if (specs != NULL)
        for (int n = 0; n < specs->SpecsCount; n++)
            printf("spec %d column %d direction %d\n", n, specs->Specs[n].ColumnIndex,
                (int)specs->Specs[n].SortDirection);

    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_sort_tristate: a sortable table with `SortTristate` and no default column can hand back zero
// specs, which is the "not sorted" state of the API.
static void DumpTablesSortTristate()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_sort_tristate\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 2,
        ImGuiTableFlags_Sortable | ImGuiTableFlags_SortTristate, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    ImGui::Dummy(ImVec2(10.0f, 10.0f));

    const ImGuiTableSortSpecs* specs = ImGui::TableGetSortSpecs();
    printf("specs %d count %d dirty %d\n", specs != NULL, specs ? specs->SpecsCount : -1,
        specs ? (int)specs->SpecsDirty : -1);

    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_column_sorted_flag: the `IsSorted` bit of `TableGetColumnFlags`, which marks the columns the
// sort specs point at.
static void DumpTablesColumnSortedFlag()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_column_sorted_flag\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 3, ImGuiTableFlags_Sortable, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed | ImGuiTableColumnFlags_DefaultSort, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    ImGui::Dummy(ImVec2(10.0f, 10.0f));
    printf("flags %08x %08x %08x\n", (unsigned)ImGui::TableGetColumnFlags(0),
        (unsigned)ImGui::TableGetColumnFlags(1), (unsigned)ImGui::TableGetColumnFlags(2));

    ImGui::EndTable();
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_after_end: the query API once the table is closed. The reference answers every getter without
// a table instead of crashing.
static void DumpTablesAfterEnd()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_after_end\n");

    ImGuiIO& io = ImGui::GetIO();
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
    ImDrawList* dl = ImGui::GetWindowDrawList();

    const bool began = ImGui::BeginTable("T", 2, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
    ImGui::TableNextColumn();
    ImGui::Dummy(ImVec2(10.0f, 10.0f));
    ImGui::EndTable();

    printf("after row %d col %d count %d hovered col %d row %d flags %08x sortable %d\n",
        ImGui::TableGetRowIndex(), ImGui::TableGetColumnIndex(), ImGui::TableGetColumnCount(),
        ImGui::TableGetHoveredColumn(), ImGui::TableGetHoveredRow(),
        (unsigned)ImGui::TableGetColumnFlags(0), ImGui::TableGetSortSpecs() != NULL);

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}
// tables_row_height: `TableNextRow(flags, min_height)` with a minimum taller than the cell contents.
// Each row is 20 px instead of the 17 its text needs, and the rows below start lower.
static void DumpTablesRowHeight()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tables_row_height\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', '0', '1', '2' };
    for (int i = 0; i < 4; i++)
        baked->FindGlyph((ImWchar)bake_order[i]);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    const ImVec2 window_pos = ImGui::GetWindowPos();

    const bool began = ImGui::BeginTable("T", 3, 0, ImVec2(0.0f, 0.0f), 0.0f);
    printf("table begin %d\n", (int)began);
    ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 60.0f);
    ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
    ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 100.0f);

    for (int row = 0; row < 3; row++)
    {
        ImGui::TableNextRow(ImGuiTableRowFlags_None, 20.0f);
        for (int col = 0; col < 3; col++)
        {
            const bool visible = ImGui::TableNextColumn();
            const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
            printf("cell %d %d visible %d cursor %08x %08x rowindex %d colindex %d\n", row, col,
                (int)visible, Bits(cursor.x), Bits(cursor.y), ImGui::TableGetRowIndex(),
                ImGui::TableGetColumnIndex());
            char text[8];
            snprintf(text, sizeof(text), "A%d%d", row, col);
            ImGui::Text("%s", text);
            const ImVec2 imin = PSub(ImGui::GetItemRectMin(), window_pos);
            const ImVec2 imax = PSub(ImGui::GetItemRectMax(), window_pos);
            printf("cell %d %d item %08x %08x %08x %08x\n", row, col, Bits(imin.x),
                Bits(imin.y), Bits(imax.x), Bits(imax.y));
        }
    }

    ImGui::EndTable();
    const ImVec2 end_cursor = PSub(ImGui::GetCursorScreenPos(), window_pos);
    printf("after cursor %08x %08x\n", Bits(end_cursor.x), Bits(end_cursor.y));

    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
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
// text_clip: the same string under different cpu_fine_clip_rect values, which trim the quads and
// their texture coordinates in software instead of leaving it to the GPU clip.
static void DumpTextClip()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario text_clip\n");

    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'H', 'e', 'l', 'o' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 4);

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
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImGuiContext& g = *GImGui;
    const ImU32 col = IM_COL32(255, 255, 255, 255);
    // No fine clip: the control, every glyph whole.
    dl->AddText(g.Font, g.FontSize, ImVec2(68.0f, 70.0f), col, "Hello", NULL, 0.0f, NULL);
    // Cut on both sides, through the middle of the first and last glyphs.
    ImVec4 clip_x(70.0f, 0.0f, 96.0f, 720.0f);
    dl->AddText(g.Font, g.FontSize, ImVec2(68.0f, 90.0f), col, "Hello", NULL, 0.0f, &clip_x);
    // Cut on both sides vertically.
    ImVec4 clip_y(0.0f, 112.0f, 1280.0f, 116.0f);
    dl->AddText(g.Font, g.FontSize, ImVec2(68.0f, 110.0f), col, "Hello", NULL, 0.0f, &clip_y);
    // Entirely below the text: the fast-forward consumes the line.
    ImVec4 clip_below(0.0f, 200.0f, 1280.0f, 300.0f);
    dl->AddText(g.Font, g.FontSize, ImVec2(68.0f, 130.0f), col, "Hello", NULL, 0.0f, &clip_below);
    // Entirely to the left of the text.
    ImVec4 clip_left(0.0f, 0.0f, 50.0f, 720.0f);
    dl->AddText(g.Font, g.FontSize, ImVec2(100.0f, 150.0f), col, "Hello", NULL, 0.0f, &clip_left);
    DumpDrawListBody(*dl);
    ImGui::End();
    ImGui::EndFrame();
}

// input_text: a single-line field across a click, a second click that moves the caret, and a click
// elsewhere that releases it. Prints the edit state and the window's draw list per frame.
static void FrameInputText(int frame, float mx, float my, bool down)
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
    static char buf[64] = "Hello world";
    ImGui::SetNextItemWidth(150.0f);
    ImGui::InputText("##field", buf, IM_ARRAYSIZE(buf));
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGuiContext& g = *GImGui;
    const ImGuiID id = w->GetID("##field");
    ImGuiInputTextState* state = ImGui::GetInputTextState(id);
    printf("frame %d active %d clicked %d", frame, (int)(g.ActiveId == id), io.MouseClicked[0] ? 1 : 0);
    if (state != NULL)
        printf(" len %d sel %d scroll %08x anim %08x", state->TextLen, state->HasSelection() ? 1 : 0,
            Bits(state->Scroll.x), Bits(state->CursorAnim));
    printf("\n");
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpInputText()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario input_text\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of "Hello world", since the reference bakes on demand.
    const char bake_order[8] = { 'H', 'e', 'l', 'o', ' ', 'w', 'r', 'd' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 8);

    // The field runs from (68,68) to (218,87). A click at x=100 lands on the caret after "Hell"
    // (28 pixels in, seven per character); a click at x=150 goes past the end of the line.
    const float steps[10][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 100.0f, 75.0f, 0.0f },
        { 100.0f, 75.0f, 1.0f },
        { 100.0f, 75.0f, 0.0f },
        { 150.0f, 75.0f, 1.0f },
        { 150.0f, 75.0f, 0.0f },
        { 75.0f, 75.0f, 1.0f },
        { 75.0f, 75.0f, 0.0f },
        { 300.0f, 200.0f, 1.0f },
        { 300.0f, 200.0f, 0.0f },
    };
    for (int f = 0; f < 10; f++)
        FrameInputText(f, steps[f][0], steps[f][1], steps[f][2] != 0.0f);
}

// input_text_edit: the keyboard half of the field. Prints the edit state and the draw list per
// frame while the test types, moves the caret, drags a selection and deletes.
static void FrameInputTextEdit(int frame, float mx, float my, bool down, const float* key_events,
    int key_count, const char* chars)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    for (int i = 0; i < key_count; i++)
        io.AddKeyEvent((ImGuiKey)(int)key_events[i * 2], key_events[i * 2 + 1] != 0.0f);
    if (chars != NULL && chars[0] != 0)
        io.AddInputCharactersUTF8(chars);

    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    static char buf[128] = "Hello world, this is a long line.";
    ImGui::SetNextItemWidth(150.0f);
    ImGui::InputText("##field", buf, IM_ARRAYSIZE(buf));
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGuiContext& g = *GImGui;
    const ImGuiID id = w->GetID("##field");
    ImGuiInputTextState* state = ImGui::GetInputTextState(id);
    printf("frame %d active %d", frame, (int)(g.ActiveId == id));
    if (state != NULL)
        printf(" len %d sel %d scroll %08x anim %08x", state->TextLen, state->HasSelection() ? 1 : 0,
            Bits(state->Scroll.x), Bits(state->CursorAnim));
    printf("\n");
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpInputTextEdit()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario input_text_edit\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of "Hello world, this is a long line." plus the ones the test types.
    const char bake_order[20] = { 'H', 'e', 'l', 'o', ' ', 'w', 'r', 'd', ',', '.',
        't', 'h', 'i', 's', 'a', 'n', 'g', 'X', 'Y', 'Z' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 20);

    const float shift_right_down[4] = { (float)ImGuiMod_Shift, 1.0f, (float)ImGuiKey_RightArrow, 1.0f };
    const float shift_right_up[4] = { (float)ImGuiMod_Shift, 0.0f, (float)ImGuiKey_RightArrow, 0.0f };
    const float left_down[2] = { (float)ImGuiKey_LeftArrow, 1.0f };
    const float left_up[2] = { (float)ImGuiKey_LeftArrow, 0.0f };
    const float home_down[2] = { (float)ImGuiKey_Home, 1.0f };
    const float home_up[2] = { (float)ImGuiKey_Home, 0.0f };
    const float right_down[2] = { (float)ImGuiKey_RightArrow, 1.0f };
    const float right_up[2] = { (float)ImGuiKey_RightArrow, 0.0f };
    const float end_down[2] = { (float)ImGuiKey_End, 1.0f };
    const float end_up[2] = { (float)ImGuiKey_End, 0.0f };
    const float delete_down[2] = { (float)ImGuiKey_Delete, 1.0f };

    // 1: a click activates the field with the caret after "Hell". 3: two characters are typed.
    // 4..7: the caret walks left twice. 8..11: Home, then one to the right. 12..14: Shift+Right
    // selects one character and typing replaces it. 15: End scrolls to the far end. 17: Home brings
    // the scroll back. 19: Delete removes the first character. 20: a click elsewhere releases.
    const float steps[21][3] = {
        { 0.0f, 0.0f, 0.0f },       // 0 idle
        { 100.0f, 75.0f, 1.0f },    // 1 activate
        { 100.0f, 75.0f, 0.0f },    // 2 release
        { 100.0f, 75.0f, 0.0f },    // 3 typing
        { 100.0f, 75.0f, 0.0f },    // 4 left
        { 100.0f, 75.0f, 0.0f },    // 5 left up
        { 100.0f, 75.0f, 0.0f },    // 6 left
        { 100.0f, 75.0f, 0.0f },    // 7 left up
        { 100.0f, 75.0f, 0.0f },    // 8 home
        { 100.0f, 75.0f, 0.0f },    // 9 home up
        { 100.0f, 75.0f, 0.0f },    // 10 right
        { 100.0f, 75.0f, 0.0f },    // 11 right up
        { 100.0f, 75.0f, 0.0f },    // 12 shift+right
        { 100.0f, 75.0f, 0.0f },    // 13 shift+right up
        { 100.0f, 75.0f, 0.0f },    // 14 typing over the selection
        { 100.0f, 75.0f, 0.0f },    // 15 end
        { 100.0f, 75.0f, 0.0f },    // 16 end up
        { 100.0f, 75.0f, 0.0f },    // 17 home
        { 100.0f, 75.0f, 0.0f },    // 18 home up
        { 100.0f, 75.0f, 0.0f },    // 19 delete
        { 300.0f, 200.0f, 1.0f },   // 20 click elsewhere
    };
    const int key_counts[21] = { 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 0, 1, 1, 1, 1, 1, 0 };
    const float* keys[21] = { NULL, NULL, NULL, NULL, left_down, left_up, left_down, left_up,
        home_down, home_up, right_down, right_up, shift_right_down, shift_right_up, NULL, end_down,
        end_up, home_down, home_up, delete_down, NULL };
    const char* chars[21] = { NULL, NULL, NULL, "XY", NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, "Z", NULL, NULL, NULL, NULL, NULL, NULL };

    for (int f = 0; f < 21; f++)
        FrameInputTextEdit(f, steps[f][0], steps[f][1], steps[f][2] != 0.0f, keys[f], key_counts[f],
            chars[f]);
}

// input_text_clipboard: Ctrl+A/C/X/V, Escape and Enter. Prints the edit state, the caller's buffer
// (so the write-back on validate is visible) and the draw list per frame.
static void FrameInputTextClipboard(int frame, float mx, float my, bool down, const float* key_events,
    int key_count, const char* chars)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    for (int i = 0; i < key_count; i++)
        io.AddKeyEvent((ImGuiKey)(int)key_events[i * 2], key_events[i * 2 + 1] != 0.0f);
    if (chars != NULL && chars[0] != 0)
        io.AddInputCharactersUTF8(chars);

    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    static char buf[128] = "Hello world";
    ImGui::SetNextItemWidth(150.0f);
    ImGui::InputText("##field", buf, IM_ARRAYSIZE(buf));
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGuiContext& g = *GImGui;
    const ImGuiID id = w->GetID("##field");
    ImGuiInputTextState* state = ImGui::GetInputTextState(id);
    printf("frame %d active %d", frame, (int)(g.ActiveId == id));
    if (state != NULL)
        printf(" len %d sel %d scroll %08x anim %08x", state->TextLen, state->HasSelection() ? 1 : 0,
            Bits(state->Scroll.x), Bits(state->CursorAnim));
    printf(" buf \"%s\"\n", buf);
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpInputTextClipboard()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario input_text_clipboard\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of "Hello world" plus the ones the test types.
    const char bake_order[12] = { 'H', 'e', 'l', 'o', ' ', 'w', 'r', 'd', 'W', 'X', 'Y', 'Z' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 12);

    const float ctrl_a_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_A, 1.0f };
    const float ctrl_a_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_A, 0.0f };
    const float ctrl_c_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_C, 1.0f };
    const float ctrl_c_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_C, 0.0f };
    const float ctrl_v_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_V, 1.0f };
    const float ctrl_v_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_V, 0.0f };
    const float ctrl_x_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_X, 1.0f };
    const float ctrl_x_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_X, 0.0f };
    const float end_down[2] = { (float)ImGuiKey_End, 1.0f };
    const float end_up[2] = { (float)ImGuiKey_End, 0.0f };
    const float esc_down[2] = { (float)ImGuiKey_Escape, 1.0f };
    const float esc_up[2] = { (float)ImGuiKey_Escape, 0.0f };
    const float enter_down[2] = { (float)ImGuiKey_Enter, 1.0f };
    const float enter_up[2] = { (float)ImGuiKey_Enter, 0.0f };

    // 1: activate with the caret after "Hell". 3: select all. 5: copy. 7: End. 9: paste (the text
    // doubles). 11: select all again. 13: cut (the text empties). 15: paste it back. 17: type a
    // character. 18: Escape, which goes back to the value the field held when it was activated.
    // 20: activate again and type. 23: Enter, which applies the edit and reports it.
    const float steps[25][3] = {
        { 0.0f, 0.0f, 0.0f },       // 0
        { 100.0f, 75.0f, 1.0f },    // 1 activate
        { 100.0f, 75.0f, 0.0f },    // 2
        { 100.0f, 75.0f, 0.0f },    // 3 Ctrl+A
        { 100.0f, 75.0f, 0.0f },    // 4
        { 100.0f, 75.0f, 0.0f },    // 5 Ctrl+C
        { 100.0f, 75.0f, 0.0f },    // 6
        { 100.0f, 75.0f, 0.0f },    // 7 End
        { 100.0f, 75.0f, 0.0f },    // 8
        { 100.0f, 75.0f, 0.0f },    // 9 Ctrl+V
        { 100.0f, 75.0f, 0.0f },    // 10
        { 100.0f, 75.0f, 0.0f },    // 11 Ctrl+A
        { 100.0f, 75.0f, 0.0f },    // 12
        { 100.0f, 75.0f, 0.0f },    // 13 Ctrl+X
        { 100.0f, 75.0f, 0.0f },    // 14
        { 100.0f, 75.0f, 0.0f },    // 15 Ctrl+V
        { 100.0f, 75.0f, 0.0f },    // 16
        { 100.0f, 75.0f, 0.0f },    // 17 type Z
        { 100.0f, 75.0f, 0.0f },    // 18 Escape
        { 100.0f, 75.0f, 0.0f },    // 19
        { 100.0f, 75.0f, 1.0f },    // 20 activate again
        { 100.0f, 75.0f, 0.0f },    // 21
        { 100.0f, 75.0f, 0.0f },    // 22 type W
        { 100.0f, 75.0f, 0.0f },    // 23 Enter
        { 100.0f, 75.0f, 0.0f },    // 24
    };
    const int key_counts[25] = { 0, 0, 0, 2, 2, 2, 2, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 0, 1, 1, 0, 0, 0, 1, 1 };
    const float* keys[25] = { NULL, NULL, NULL, ctrl_a_down, ctrl_a_up, ctrl_c_down, ctrl_c_up,
        end_down, end_up, ctrl_v_down, ctrl_v_up, ctrl_a_down, ctrl_a_up, ctrl_x_down, ctrl_x_up,
        ctrl_v_down, ctrl_v_up, NULL, esc_down, esc_up, NULL, NULL, NULL, enter_down, enter_up };
    const char* chars[25] = { NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, NULL, "Z", NULL, NULL, NULL, NULL, "W", NULL, NULL };

    for (int f = 0; f < 25; f++)
        FrameInputTextClipboard(f, steps[f][0], steps[f][1], steps[f][2] != 0.0f, keys[f],
            key_counts[f], chars[f]);
}

// input_text_mouse_undo: the drag selection, the double-click word selection and Ctrl+Z/Y/Shift+Z.
static void FrameInputTextMouseUndo(int frame, float mx, float my, bool down, const float* key_events,
    int key_count, const char* chars)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    // The reference derives MouseDelta from these two, so moving the mouse is enough to drag.
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    for (int i = 0; i < key_count; i++)
        io.AddKeyEvent((ImGuiKey)(int)key_events[i * 2], key_events[i * 2 + 1] != 0.0f);
    if (chars != NULL && chars[0] != 0)
        io.AddInputCharactersUTF8(chars);

    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    ImGui::Begin("W", NULL, flags);
    static char buf[128] = "Hello world";
    ImGui::SetNextItemWidth(150.0f);
    ImGui::InputText("##field", buf, IM_ARRAYSIZE(buf));
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGuiContext& g = *GImGui;
    const ImGuiID id = w->GetID("##field");
    ImGuiInputTextState* state = ImGui::GetInputTextState(id);
    printf("frame %d active %d", frame, (int)(g.ActiveId == id));
    if (state != NULL)
        printf(" len %d sel %d cursor %d scroll %08x anim %08x", state->TextLen,
            state->HasSelection() ? 1 : 0, state->GetCursorPos(), Bits(state->Scroll.x),
            Bits(state->CursorAnim));
    printf(" buf \"%s\"\n", buf);
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpInputTextMouseUndo()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario input_text_mouse_undo\n");

    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[10] = { 'H', 'e', 'l', 'o', ' ', 'w', 'r', 'd', 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 10);

    const float ctrl_z_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_Z, 1.0f };
    const float ctrl_z_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_Z, 0.0f };
    const float ctrl_y_down[4] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiKey_Y, 1.0f };
    const float ctrl_y_up[4] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiKey_Y, 0.0f };
    const float ctrl_shift_z_down[6] = { (float)ImGuiMod_Ctrl, 1.0f, (float)ImGuiMod_Shift, 1.0f, (float)ImGuiKey_Z, 1.0f };
    const float ctrl_shift_z_up[6] = { (float)ImGuiMod_Ctrl, 0.0f, (float)ImGuiMod_Shift, 0.0f, (float)ImGuiKey_Z, 0.0f };

    // 1: activate. 3: type two characters, so there are two edits to undo. 4: a click at x=85.
    // 5-6: the button is held while the mouse moves right, which drags the selection. 8-10: two
    // clicks in the same place, the second one selecting the word under the mouse. 12-14: two undos,
    // one per typed character. 16: redo. 18: redo with Ctrl+Shift+Z. 20: release the field.
    const float steps[22][3] = {
        { 0.0f, 0.0f, 0.0f },      // 0
        { 100.0f, 75.0f, 1.0f },   // 1 activate
        { 100.0f, 75.0f, 0.0f },   // 2
        { 100.0f, 75.0f, 0.0f },   // 3 type AB
        { 85.0f, 75.0f, 1.0f },    // 4 click
        { 130.0f, 75.0f, 1.0f },   // 5 drag
        { 160.0f, 75.0f, 1.0f },   // 6 drag
        { 160.0f, 75.0f, 0.0f },   // 7 up
        { 110.0f, 75.0f, 1.0f },   // 8 click 1
        { 110.0f, 75.0f, 0.0f },   // 9
        { 110.0f, 75.0f, 1.0f },   // 10 click 2
        { 110.0f, 75.0f, 0.0f },   // 11
        { 110.0f, 75.0f, 0.0f },   // 12 Ctrl+Z
        { 110.0f, 75.0f, 0.0f },   // 13
        { 110.0f, 75.0f, 0.0f },   // 14 Ctrl+Z
        { 110.0f, 75.0f, 0.0f },   // 15
        { 110.0f, 75.0f, 0.0f },   // 16 Ctrl+Y
        { 110.0f, 75.0f, 0.0f },   // 17
        { 110.0f, 75.0f, 0.0f },   // 18 Ctrl+Shift+Z
        { 110.0f, 75.0f, 0.0f },   // 19
        { 300.0f, 200.0f, 1.0f },  // 20 click elsewhere
        { 300.0f, 200.0f, 0.0f },  // 21
    };
    const int key_counts[22] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 3, 3, 0, 0 };
    const float* keys[22] = { NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
        ctrl_z_down, ctrl_z_up, ctrl_z_down, ctrl_z_up, ctrl_y_down, ctrl_y_up, ctrl_shift_z_down,
        ctrl_shift_z_up, NULL, NULL };
    const char* chars[22] = { NULL, NULL, NULL, "AB", NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL };

    for (int f = 0; f < 22; f++)
        FrameInputTextMouseUndo(f, steps[f][0], steps[f][1], steps[f][2] != 0.0f, keys[f],
            key_counts[f], chars[f]);
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

// windows_focus: `SetNextWindowFocus` raises a window to the front of the display order and gives it
// the focus without a click, so the draw order (and the title bar colour) swap.
static void FrameTwoWindowsFocus(int focus)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar;
    if (focus == 0)
        ImGui::SetNextWindowFocus();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("A", NULL, flags);
    ImGui::End();
    if (focus == 1)
        ImGui::SetNextWindowFocus();
    ImGui::SetNextWindowPos(ImVec2(110.0f, 100.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("B", NULL, flags);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsFocus()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_focus\n");
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

    // A = (60,60)-(360,260), B = (110,100)-(410,300), B created last so it starts on top.
    for (int i = 0; i < 6; i++)
        FrameTwoWindowsFocus(-1);
    WriteGlyphFixture(font, bake_order, 2);
    int frame = 0;

    FrameTwoWindowsFocus(-1);
    DumpWindowOrderFrame(frame++, "idle");
    FrameTwoWindowsFocus(0);
    DumpWindowOrderFrame(frame++, "focus_a");
    FrameTwoWindowsFocus(-1);
    DumpWindowOrderFrame(frame++, "idle_a_on_top");
    FrameTwoWindowsFocus(1);
    DumpWindowOrderFrame(frame++, "focus_b");
    FrameTwoWindowsFocus(-1);
    DumpWindowOrderFrame(frame++, "idle_b_on_top");
    FrameTwoWindowsFocus(0);
    DumpWindowOrderFrame(frame++, "focus_a_again");
    FrameTwoWindowsFocus(-1);
    DumpWindowOrderFrame(frame++, "idle_a_again");
}

// windows_collapse_api: `SetNextWindowCollapsed` collapses and expands a window without touching its
// title bar arrow. A collapsed window draws only its bar and skips the items submitted into it.
static void FrameCollapseApi(int collapsed_state)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (collapsed_state >= 0)
        ImGui::SetNextWindowCollapsed(collapsed_state != 0);
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar;
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("C", NULL, flags);
    ImGui::Button("B");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsCollapseApi()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_collapse_api\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'C', 'B' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);

    for (int i = 0; i < 6; i++)
        FrameCollapseApi(-1);
    // The collapse arrow is an anti-aliased stroke, so the fixture carries the line rows too.
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 2);
    int frame = 0;

    FrameCollapseApi(-1);
    DumpWindowOrderFrame(frame++, "expanded");
    FrameCollapseApi(1);
    DumpWindowOrderFrame(frame++, "collapse_api");
    FrameCollapseApi(-1);
    DumpWindowOrderFrame(frame++, "collapsed");
    FrameCollapseApi(0);
    DumpWindowOrderFrame(frame++, "expand_api");
    FrameCollapseApi(-1);
    DumpWindowOrderFrame(frame++, "expanded_again");
}

// windows_set_pos: `SetWindowPos` by name moves a live window immediately (the port's remembered
// position), truncating the coordinates; it stays there on the following frames.
static void FrameSetWindowPos(int set_pos, float x, float y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (set_pos)
        ImGui::SetWindowPos("P", ImVec2(x, y));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("P", NULL, flags);
    ImGui::Button("B");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsSetPos()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_set_pos\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'P', 'B' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 2);

    // The first frames place the window explicitly so both sides start from the same memory.
    for (int i = 0; i < 2; i++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("P", NULL, ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
            ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse);
        ImGui::Button("B");
        ImGui::End();
        ImGui::EndFrame();
    }
    int frame = 0;

    FrameSetWindowPos(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "idle");
    FrameSetWindowPos(1, 120.4f, 90.7f);
    DumpWindowOrderFrame(frame++, "set_pos");
    FrameSetWindowPos(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "stays");
    FrameSetWindowPos(1, 50.0f, 40.0f);
    DumpWindowOrderFrame(frame++, "set_pos_again");
    FrameSetWindowPos(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "stays_again");
}

// windows_set_size: `SetWindowSize` by name resizes a live window immediately (the port's remembered
// size), truncating the axis; it stays that size on the following frames.
static void FrameSetWindowSize(int set_size, float x, float y)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (set_size)
        ImGui::SetWindowSize("P", ImVec2(x, y));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("P", NULL, flags);
    ImGui::Button("B");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsSetSize()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_set_size\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'P', 'B' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 2);

    for (int i = 0; i < 2; i++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("P", NULL, ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
            ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse);
        ImGui::Button("B");
        ImGui::End();
        ImGui::EndFrame();
    }
    int frame = 0;

    FrameSetWindowSize(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "idle");
    FrameSetWindowSize(1, 200.9f, 120.7f);
    DumpWindowOrderFrame(frame++, "set_size");
    FrameSetWindowSize(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "stays");
    FrameSetWindowSize(1, 300.0f, 200.0f);
    DumpWindowOrderFrame(frame++, "set_size_again");
    FrameSetWindowSize(0, 0.0f, 0.0f);
    DumpWindowOrderFrame(frame++, "stays_again");
}

// separator: `Separator` between three labels. The line spans the window's work width, is as thick
// as `Style::SeparatorSize` and is laid out as an item of that thickness.
static void DumpSeparator()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario separator\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::Separator();
        ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d sep0 item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::Text("B");
        ImGui::Separator();
        mn = PSub(ImGui::GetItemRectMin(), wpos);
        mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d sep1 item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::Text("C");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// separator_text: `SeparatorText` between two labels. The item is as wide as the window's work width
// and as tall as the label plus SeparatorTextPadding, with a line on each side of the label.
static void DumpSeparatorText()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario separator_text\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'S', 'B' };
    for (char ch : bake_order)
        baked->FindGlyph((ImWchar)ch);
    WriteGlyphFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::SeparatorText("S");
        ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d sep item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::Text("B");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// menu_bar_separator: a `Separator` inside a menu bar is the vertical form: a line as tall as the
// line the bar already laid out, one pixel wide, that only advances the horizontal layout.
static void FrameMenuBarSeparator(int frame)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar |
        ImGuiWindowFlags_MenuBar);
    if (ImGui::BeginMenuBar())
    {
        ImGui::MenuItem("File");
        ImGui::Separator();
        const ImVec2 wpos = ImGui::GetWindowPos();
        const ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        const ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        const ImVec2 cursor = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d sep item %08x %08x %08x %08x cursor %08x %08x\n", frame, Bits(mn.x),
            Bits(mn.y), Bits(mx.x), Bits(mx.y), Bits(cursor.x), Bits(cursor.y));
        ImGui::MenuItem("Edit");
        ImGui::EndMenuBar();
    }
    ImGui::Button("x");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpMenuBarSeparator()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario menu_bar_separator\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every string, including the button's label, which the reference bakes on demand.
    const char bake_order[8] = { 'F', 'i', 'l', 'e', 'E', 'd', 't', 'x' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 8);

    for (int f = 0; f < 5; f++)
    {
        FrameMenuBarSeparator(f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}

// disabled: `BeginDisabled`/`EndDisabled`. Everything inside is drawn with `Style::Alpha` scaled by
// `Style::DisabledAlpha` and does not hover nor press; the button after `EndDisabled` is back to normal.
static void FrameDisabled(float mx, float my, bool down, int frame)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGui::Text("A");
    const bool b = ImGui::Button("B");
    ImGui::BeginDisabled();
    ImGui::Text("C");
    const bool d = ImGui::Button("D");
    ImGui::EndDisabled();
    const bool e = ImGui::Button("E");
    printf("frame %d pressed b %d d %d e %d\n", frame, b ? 1 : 0, d ? 1 : 0, e ? 1 : 0);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpDisabled()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario disabled\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[5] = { 'A', 'B', 'C', 'D', 'E' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 5);

    // Idle, press and release over the disabled button, then over the enabled one.
    const float steps[4][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 70.0f, 130.0f, 1.0f },
        { 70.0f, 130.0f, 0.0f },
        { 70.0f, 150.0f, 0.0f },
    };
    for (int frame = 0; frame < 4; frame++)
    {
        FrameDisabled(steps[frame][0], steps[frame][1], steps[frame][2] != 0.0f, frame);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", frame, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}

// bullet: `Bullet` between two labels. The disc is `FontSize * 0.2` around a point inside an item as
// wide as the font, and the label after it stays on the same line, indented by the bullet.
static void DumpBullet()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario bullet\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::Bullet();
        const ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        const ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        const ImVec2 cur = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d bullet item %08x %08x %08x %08x cursor %08x %08x\n", frame, Bits(mn.x),
            Bits(mn.y), Bits(mx.x), Bits(mx.y), Bits(cur.x), Bits(cur.y));
        ImGui::Text("B");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// progress_bar: the bar's frame, its filled part, the percentage overlay placed after the fill and
// clipped to the bar, the default item width when no size is asked for, and the indeterminate slice.
static void DumpProgressBar()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario progress_bar\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every string: the digits and the percent sign of the default overlay, plus the
    // letters of the custom one, which the reference would otherwise bake on demand.
    const char bake_order[15] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '%', 'h', 'a', 'l', 'f' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 15);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        // The fractions are chosen so every filled edge lands on a whole pixel: a fractional one makes
        // the reference take AddRectFilled's anti-aliased path, which the port does not model yet.
        const char* names[4] = { "quarter", "three_quarters", "custom_overlay", "default_width" };
        const float fractions[4] = { 0.25f, 0.75f, 0.5f, 0.5f };
        for (int i = 0; i < 4; i++)
        {
            if (i == 2)
                ImGui::ProgressBar(fractions[i], ImVec2(200.0f, 19.0f), "half##bar");
            else if (i == 3)
                ImGui::ProgressBar(fractions[i]);
            else
                ImGui::ProgressBar(fractions[i], ImVec2(200.0f, 19.0f));
            const ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
            const ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
            printf("frame %d %s item %08x %08x %08x %08x\n", frame, names[i], Bits(mn.x),
                Bits(mn.y), Bits(mx.x), Bits(mx.y));
        }
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// text_colors: `Text` keeps the `ImGuiCol_Text` colour, `TextColored` pushes an explicit one and
// `TextDisabled` uses `ImGuiCol_TextDisabled`; the four labels show the difference in the draw list.
static void DumpTextColors()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario text_colors\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { 'A', 'B', 'C', 'D' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 4);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        ImGui::Text("A");
        ImGui::TextColored(ImVec4(1.0f, 0.0f, 0.0f, 1.0f), "B");
        ImGui::TextDisabled("C");
        ImGui::Text("D");
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// label_text: `LabelText` puts a value in an item as wide as `CalcItemWidth`, with its label after
// `ItemInnerSpacing.x` on the same line and both `FramePadding` down from the item's top.
static void DumpLabelText()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario label_text\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[4] = { '4', '2', '7', 'N' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 4);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::LabelText("N", "42");
        ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d labelled item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::LabelText("", "7");
        mn = PSub(ImGui::GetItemRectMin(), wpos);
        mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d bare item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// bullet_text: `BulletText` is one item that holds the disc and the text, with the text starting at
// FontSize + FramePadding.x * 2 from the item's left edge.
static void DumpBulletText()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario bullet_text\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        const char* names[2] = { "first", "second" };
        const char* texts[2] = { "A", "B" };
        for (int i = 0; i < 2; i++)
        {
            ImGui::BulletText(texts[i]);
            const ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
            const ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
            printf("frame %d %s item %08x %08x %08x %08x\n", frame, names[i], Bits(mn.x),
                Bits(mn.y), Bits(mx.x), Bits(mx.y));
        }
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// separator_text_empty: `SeparatorText("")` has no label, so it draws a single line across the item
// instead of the two lines that surround a label.
static void DumpSeparatorTextEmpty()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario separator_text_empty\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_NoBackground;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("S", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::SeparatorText("");
        const ImVec2 mn = PSub(ImGui::GetItemRectMin(), wpos);
        const ImVec2 mx = PSub(ImGui::GetItemRectMax(), wpos);
        printf("frame %d sep item %08x %08x %08x %08x\n", frame, Bits(mn.x), Bits(mn.y),
            Bits(mx.x), Bits(mx.y));
        ImGui::Text("B");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// spacing: `Spacing` is an empty item: it closes the line and leaves one `ItemSpacing.y` gap, so the
// label after it starts a further line down.
static void DumpSpacing()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario spacing\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::Spacing();
        const ImVec2 after_spacing = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after spacing %08x %08x\n", frame, Bits(after_spacing.x),
            Bits(after_spacing.y));
        ImGui::Text("B");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// new_line: `NewLine` ends the line. On an empty line it reserves one font height; after items it
// keeps their height, so the cursor moves below them.
static void DumpNewLine()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario new_line\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::NewLine();
        const ImVec2 on_empty = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d on empty %08x %08x\n", frame, Bits(on_empty.x), Bits(on_empty.y));
        ImGui::Text("A");
        const ImVec2 after_a = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after a %08x %08x\n", frame, Bits(after_a.x), Bits(after_a.y));
        ImGui::NewLine();
        const ImVec2 after_line = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after newline %08x %08x\n", frame, Bits(after_line.x), Bits(after_line.y));
        ImGui::Text("B");
        const ImVec2 end = PSub(ImGui::GetCursorScreenPos(), wpos);
        printf("frame %d after cursor %08x %08x\n", frame, Bits(end.x), Bits(end.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// indent: Indent moves the layout right by IndentSpacing (21) and Unindent brings it back, so the
// labels after each call start at a different x.
static void DumpIndent()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario indent\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[3] = { 'A', 'B', 'C' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 3);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const ImVec2 wpos = ImGui::GetWindowPos();
        ImGui::Text("A");
        ImGui::Indent();
        ImGui::Text("B");
        ImGui::Unindent();
        ImGui::Text("C");
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// invisible_button: the widget draws nothing, it only takes an item of the given size; the return
// value is the proof, and the draw list holds nothing but the window.
static void DumpInvisibleButton()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario invisible_button\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[1] = { 'A' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 1);

    const float steps[4][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 70.0f, 70.0f, 1.0f },
        { 70.0f, 70.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
    };
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 4; frame++)
    {
        io.DeltaTime = 0.1f;
        io.MousePos = ImVec2(steps[frame][0], steps[frame][1]);
        io.MouseDown[0] = steps[frame][2] != 0.0f;
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        const bool pressed = ImGui::InvisibleButton("hit", ImVec2(50.0f, 20.0f));
        ImGui::Text("A");
        ImGui::End();
        printf("frame %d pressed %d\n", frame, pressed ? 1 : 0);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", frame, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
        ImGui::EndFrame();
    }
}

// windows_set_collapsed: SetWindowCollapsed by name applies the state at the window's next Begin, the
// remembered one, so the bar-only frame comes without touching the title bar arrow.
static void FrameSetWindowCollapsed(int collapsed_state)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (collapsed_state >= 0)
        ImGui::SetWindowCollapsed("C", collapsed_state != 0);
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoScrollbar;
    ImGui::Begin("C", NULL, flags);
    ImGui::Button("B");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowsSetCollapsed()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_set_collapsed\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'C', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);

    // The first frames place the window explicitly so both sides start from the same memory.
    for (int i = 0; i < 6; i++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("C", NULL, ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
            ImGuiWindowFlags_NoScrollbar);
        ImGui::Button("B");
        ImGui::End();
        ImGui::EndFrame();
    }
    ImGui::NewFrame();
    ImGui::EndFrame();
    WriteWindowIconsFixture(font, bake_order, 2);
    int frame = 0;

    FrameSetWindowCollapsed(-1);
    DumpWindowOrderFrame(frame++, "expanded");
    FrameSetWindowCollapsed(1);
    DumpWindowOrderFrame(frame++, "collapse_api");
    FrameSetWindowCollapsed(-1);
    DumpWindowOrderFrame(frame++, "collapsed");
    FrameSetWindowCollapsed(0);
    DumpWindowOrderFrame(frame++, "expand_api");
    FrameSetWindowCollapsed(-1);
    DumpWindowOrderFrame(frame++, "expanded_again");
}

// windows_set_focus: SetWindowFocus by name raises the window and gives it the focus at once, so the
// draw order swaps on the same frame, like a click but without one.
static void FrameTwoWindowsSetFocus(int focus)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (focus == 0)
        ImGui::SetWindowFocus("A");
    else if (focus == 1)
        ImGui::SetWindowFocus("B");
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

static void DumpWindowsSetFocus()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario windows_set_focus\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    for (int i = 0; i < 6; i++)
        FrameTwoWindowsSetFocus(-1);
    int frame = 0;

    FrameTwoWindowsSetFocus(-1);
    DumpWindowOrderFrame(frame++, "idle");
    FrameTwoWindowsSetFocus(0);
    DumpWindowOrderFrame(frame++, "focus_a");
    FrameTwoWindowsSetFocus(-1);
    DumpWindowOrderFrame(frame++, "idle_a_on_top");
    FrameTwoWindowsSetFocus(1);
    DumpWindowOrderFrame(frame++, "focus_b");
    FrameTwoWindowsSetFocus(-1);
    DumpWindowOrderFrame(frame++, "idle_b_on_top");
}

// cursor_pos: GetCursorPos reports the cursor in window coordinates and SetCursorPos places it
// there, so a text after it starts at the given point.
static void DumpCursorPos()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario cursor_pos\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[2] = { 'A', 'B' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 2);

    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse;
    for (int frame = 0; frame < 2; frame++)
    {
        ImGui::NewFrame();
        ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
        ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
        ImGui::Begin("W", NULL, flags);
        ImDrawList* dl = ImGui::GetWindowDrawList();
        ImVec2 p = ImGui::GetCursorPos();
        printf("frame %d start %08x %08x\n", frame, Bits(p.x), Bits(p.y));
        ImGui::Text("A");
        p = ImGui::GetCursorPos();
        printf("frame %d after a %08x %08x\n", frame, Bits(p.x), Bits(p.y));
        ImGui::SetCursorPos(ImVec2(10.0f, 5.0f));
        p = ImGui::GetCursorPos();
        printf("frame %d set %08x %08x\n", frame, Bits(p.x), Bits(p.y));
        ImGui::Text("B");
        p = ImGui::GetCursorPos();
        printf("frame %d after b %08x %08x\n", frame, Bits(p.x), Bits(p.y));
        ImGui::End();
        if (frame == 1)
            DumpDrawListBody(*dl);
        ImGui::EndFrame();
    }
}

// window_bg_alpha: SetNextWindowBgAlpha paints the next window background with that alpha instead
// of the full one, so its first quad comes out translucent.
static void FrameBgAlpha(bool with_alpha)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    if (with_alpha)
        ImGui::SetNextWindowBgAlpha(0.5f);
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoCollapse);
    ImGui::Text("A");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowBgAlpha()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_bg_alpha\n");
    ImGuiIO& io = ImGui::GetIO();
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[1] = { 'A' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 1);

    const char* tags[2] = { "opaque", "half" };
    for (int frame = 0; frame < 2; frame++)
    {
        FrameBgAlpha(frame == 1);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d %s lists %d\n", frame, tags[frame], dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
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

// window_hscrollbar: a window whose content is too wide, so the horizontal scrollbar shows up. It
// takes its height off the bottom of the interior and can be grabbed and dragged along.
static void FrameHScrollWindow(float scroll_x, float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    if (scroll_x >= 0.0f)
        ImGui::SetNextWindowScroll(ImVec2(scroll_x, 0.0f));
    ImGui::Begin("H", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse |
        ImGuiWindowFlags_HorizontalScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(400.0f, 100.0f));
    const ImRect bar = ImGui::GetWindowScrollbarRect(w, ImGuiAxis_X);
    printf("frame scroll %08x max %08x %08x inner %08x %08x %08x %08x bar %08x %08x %08x %08x sb %d %d\n",
        Bits(w->Scroll.x), Bits(w->ScrollMax.x), Bits(w->ScrollMax.y), Bits(w->InnerRect.Min.x),
        Bits(w->InnerRect.Min.y), Bits(w->InnerRect.Max.x), Bits(w->InnerRect.Max.y), Bits(bar.Min.x),
        Bits(bar.Min.y), Bits(bar.Max.x), Bits(bar.Max.y), w->ScrollbarX ? 1 : 0,
        w->ScrollbarY ? 1 : 0);
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowHScrollbar()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_hscrollbar\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // No content size on the first frame, so the bar appears on the second.
    FrameHScrollWindow(-1.0f, 0.0f, 0.0f, false);
    FrameHScrollWindow(-1.0f, 0.0f, 0.0f, false);
    // Scrolled by API: the grab moves right.
    FrameHScrollWindow(60.0f, 0.0f, 0.0f, false);
    // Grab the bar and drag it, then release.
    FrameHScrollWindow(-1.0f, 100.0f, 253.0f, false);
    FrameHScrollWindow(-1.0f, 100.0f, 253.0f, true);
    FrameHScrollWindow(-1.0f, 140.0f, 253.0f, true);
    FrameHScrollWindow(-1.0f, 140.0f, 253.0f, false);
    FrameHScrollWindow(-1.0f, 0.0f, 0.0f, false);
}

// window_scrollbar_page: `ConfigScrollbarScrollByPage` makes a click on the bar's track scroll a
// page instead of jumping to the clicked point. Shift forces the same with the option off.
//
// The content is 1000 tall in a 200 tall window, so the grab is short (38 px out of 194) and a click
// below it lands far from the bottom: a page is 200, the absolute seek would be 542.
static void FramePageWindow(int scroll_y, bool by_page, bool shift, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(352.0f, my);
    io.MouseDown[0] = down;
    io.ConfigScrollbarScrollByPage = by_page;
    io.AddKeyEvent(ImGuiMod_Shift, shift);
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    if (scroll_y >= 0)
        ImGui::SetNextWindowScroll(ImVec2(0.0f, (float)scroll_y));
    const ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoCollapse;
    ImGui::Begin("P", NULL, flags);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, 1000.0f));
    printf("frame scroll %08x max %08x\n", Bits(w->Scroll.y), Bits(w->ScrollMax.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowScrollbarPage()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_scrollbar_page\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    for (int i = 0; i < 4; i++)
        FramePageWindow(-1, false, false, 0.0f, false);

    // By page, click below the grab: one page down (200), not the absolute 542.
    FramePageWindow(-1, true, false, 200.0f, false);
    FramePageWindow(-1, true, false, 200.0f, true);
    FramePageWindow(-1, true, false, 200.0f, false);
    FramePageWindow(-1, true, false, 200.0f, false);
    // Again from the same place: another page (400).
    FramePageWindow(-1, true, false, 200.0f, true);
    FramePageWindow(-1, true, false, 200.0f, false);
    // Shift forces the page mode with the option off: 400 + 200.
    FramePageWindow(-1, false, true, 200.0f, false);
    FramePageWindow(-1, false, true, 200.0f, true);
    FramePageWindow(-1, false, true, 200.0f, false);
    // Option and Shift both off: the click seeks to where it landed (542).
    FramePageWindow(0, false, false, 200.0f, false);
    FramePageWindow(0, false, false, 200.0f, true);
    FramePageWindow(0, false, false, 200.0f, false);
}

// window_min_size: `WindowMinSize` clamps a size asked for by API. It is deliberately *not* applied
// to an auto-fitted size, where the content plus padding wins even when it is smaller.
static void FrameMinSizeWindow(float w, float h, bool auto_resize, bool dump)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGuiWindowFlags flags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize;
    if (auto_resize)
    {
        // An auto-sized window ignores the requested size.
        ImGui::SetNextWindowSize(ImVec2(10.0f, 10.0f));
        flags |= ImGuiWindowFlags_AlwaysAutoResize;
    }
    else
    {
        ImGui::SetNextWindowSize(ImVec2(w, h));
    }
    ImGui::Begin("M", NULL, flags);
    ImGuiWindow* win = ImGui::GetCurrentWindow();
    if (auto_resize)
        ImGui::Dummy(ImVec2(1.0f, 1.0f));
    if (!dump)
    {
        ImGui::End();
        ImGui::EndFrame();
        return;
    }
    printf("frame requested %08x %08x auto %d size %08x %08x min %08x %08x\n", Bits(w), Bits(h),
        auto_resize ? 1 : 0, Bits(win->Size.x), Bits(win->Size.y),
        Bits(ImGui::GetStyle().WindowMinSize.x), Bits(ImGui::GetStyle().WindowMinSize.y));
    DumpDrawListBody(*win->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowMinSize()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_min_size\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // A tiny requested size is bumped to the minimum.
    FrameMinSizeWindow(10.0f, 10.0f, false, true);
    FrameMinSizeWindow(10.0f, 10.0f, false, true);
    // Just under: also bumped.
    FrameMinSizeWindow(31.0f, 31.0f, false, true);
    // Exactly at the minimum: unchanged.
    FrameMinSizeWindow(32.0f, 32.0f, false, true);
    // Above it: unchanged.
    FrameMinSizeWindow(120.0f, 80.0f, false, true);
    FrameMinSizeWindow(10.0f, 10.0f, true, true);
}

// window_resize_grips: the two lower corner grips of a resizable window. The lower-right one is
// visible at rest; the lower-left one only while it is hovered or held. Resizing is not wired up in
// the port yet, so this scenario only pins the drawing and the colours.
static void FrameGripWindow(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("G", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    printf("frame size %08x %08x\n", Bits(w->Size.x), Bits(w->Size.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowResizeGrips()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_resize_grips\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // Away from the window: only the lower-right grip is visible, faintly.
    for (int i = 0; i < 3; i++)
        FrameGripWindow(0.0f, 0.0f, false);
    // Over the lower-right grip (the window ends at 360,260): the handle reaches 12 pixels out.
    FrameGripWindow(357.0f, 257.0f, false);
    FrameGripWindow(357.0f, 257.0f, true);
    FrameGripWindow(357.0f, 257.0f, false);
    // Over the lower-left grip: the second one shows up too.
    FrameGripWindow(63.0f, 257.0f, false);
    FrameGripWindow(0.0f, 0.0f, false);
}

// window_resize_drag: dragging the lower-right resize grip changes the window size. The size is
// computed from the mouse position and the offset inside the handle, so a drag is a delta and the
// size constraint applies.
static void FrameResizeDrag(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("R", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    printf("frame size %08x %08x pos %08x %08x\n", Bits(w->Size.x), Bits(w->Size.y),
        Bits(w->Pos.x), Bits(w->Pos.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowResizeDrag()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_resize_drag\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    for (int i = 0; i < 4; i++)
        FrameResizeDrag(0.0f, 0.0f, false);
    // The window ends at (360, 260); the handle reaches 12 pixels out and 10 in.
    FrameResizeDrag(357.0f, 257.0f, false);
    FrameResizeDrag(357.0f, 257.0f, true);
    FrameResizeDrag(377.0f, 277.0f, true);
    FrameResizeDrag(397.0f, 297.0f, true);
    FrameResizeDrag(397.0f, 297.0f, false);
    FrameResizeDrag(0.0f, 0.0f, false);
}

// window_resize_edges: the lower-left grip, which moves the window as well as resizing it, and the
// four borders as grab areas. Each drag is a single axis, so the position and size changes are easy
// to read in the dump.
static void FrameResizeEdge(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("E", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    printf("frame size %08x %08x pos %08x %08x\n", Bits(w->Size.x), Bits(w->Size.y),
        Bits(w->Pos.x), Bits(w->Pos.y));
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowResizeEdges()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_resize_edges\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    // The line table only reaches the shared data once a frame has selected the atlas font.
    ImGui::NewFrame();
    WriteLineUvFixture();
    ImGui::EndFrame();

    for (int i = 0; i < 4; i++)
        FrameResizeEdge(0.0f, 0.0f, false);

    // Lower-left grip (the window is 60,60 to 360,260): drag left by twenty.
    FrameResizeEdge(63.0f, 257.0f, false);
    FrameResizeEdge(63.0f, 257.0f, true);
    FrameResizeEdge(43.0f, 257.0f, true);
    FrameResizeEdge(43.0f, 257.0f, false);

    // Right border: drag right by twenty, which only grows the width.
    FrameResizeEdge(362.0f, 150.0f, false);
    FrameResizeEdge(362.0f, 150.0f, true);
    FrameResizeEdge(382.0f, 150.0f, true);
    FrameResizeEdge(382.0f, 150.0f, false);

    // Bottom border: drag down by twenty, which only grows the height.
    FrameResizeEdge(200.0f, 262.0f, false);
    FrameResizeEdge(200.0f, 262.0f, true);
    FrameResizeEdge(200.0f, 282.0f, true);
    FrameResizeEdge(200.0f, 282.0f, false);
}

// window_resize_cursor: the mouse cursor the handles ask for. The reference sets it every frame and
// resets it to the arrow at the top of the frame, so each line is what that frame ended with.
static void FrameResizeCursor(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("C", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar);
    printf("frame cursor %d\n", (int)ImGui::GetMouseCursor());
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowResizeCursor()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_resize_cursor\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    // Hovering a border strokes it, which samples the atlas line texture: pin the table here too.
    ImGui::NewFrame();
    WriteLineUvFixture();
    ImGui::EndFrame();

    for (int i = 0; i < 3; i++)
        FrameResizeCursor(0.0f, 0.0f, false);
    // The lower-right grip, then the lower-left one.
    FrameResizeCursor(357.0f, 257.0f, false);
    FrameResizeCursor(357.0f, 257.0f, true);
    FrameResizeCursor(63.0f, 257.0f, false);
    // The right border, then the bottom border.
    FrameResizeCursor(362.0f, 150.0f, false);
    FrameResizeCursor(362.0f, 150.0f, false);
    FrameResizeCursor(200.0f, 262.0f, false);
    // Back to nothing.
    FrameResizeCursor(200.0f, 150.0f, false);
}

// window_resize_autofit: double-clicking a resize grip fits the window to its content. The reference
// asks CalcWindowAutoFitSize for both axes and then applies the size constraint, so the result is the
// content plus the window padding, never under WindowMinSize. The content here is 100 by 50, so the
// fitted size is 116 by 66.
static void FrameResizeAutofit(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("A", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    ImGui::Dummy(ImVec2(100.0f, 50.0f));
    printf("frame size %08x %08x pos %08x %08x\n", Bits(w->Size.x), Bits(w->Size.y),
        Bits(w->Pos.x), Bits(w->Pos.y));
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpWindowResizeAutofit()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario window_resize_autofit\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    // Settle: the content size of the previous frame is what the fit will use.
    for (int i = 0; i < 4; i++)
        FrameResizeAutofit(0.0f, 0.0f, false);
    // A single click on the lower-right grip: no fit, the size is asked for again every frame.
    FrameResizeAutofit(357.0f, 257.0f, false);
    FrameResizeAutofit(357.0f, 257.0f, true);
    FrameResizeAutofit(357.0f, 257.0f, false);
    // Now a double click within the 0.30 s threshold: press, release, press.
    FrameResizeAutofit(357.0f, 257.0f, true);
    FrameResizeAutofit(357.0f, 257.0f, false);
    FrameResizeAutofit(357.0f, 257.0f, true);
    FrameResizeAutofit(357.0f, 257.0f, false);
}

// collapsing_header: a framed collapsing header. A click on the label toggles it on release, a click
// on the arrow toggles it on press, and the arrow points down while open and right while closed.
static void FrameCollapsingHeader(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("H", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    bool first = ImGui::CollapsingHeader("First");
    if (first)
        ImGui::Dummy(ImVec2(20.0f, 10.0f));
    bool second = ImGui::CollapsingHeader("Second");
    if (second)
        ImGui::Dummy(ImVec2(20.0f, 10.0f));
    printf("frame first %d second %d\n", first ? 1 : 0, second ? 1 : 0);
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpCollapsingHeader()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario collapsing_header\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter the two labels use, so the fixture carries all of them.
    const char bake_order[11] = { 'F', 'i', 'r', 's', 't', 'S', 'e', 'c', 'o', 'n', 'd' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    // The header draws the collapse arrow as a stroke, so the fixture carries the line rows too.
    ImGui::NewFrame();
    WriteWindowIconsFixture(font, bake_order, 11);
    ImGui::EndFrame();

    // The headers sit at the top of the window: the first spans y 68 to 87, the second follows.
    for (int i = 0; i < 3; i++)
        FrameCollapsingHeader(0.0f, 0.0f, false);
    // A click on the label of the first one: press, then release inside it opens it.
    FrameCollapsingHeader(200.0f, 78.0f, false);
    FrameCollapsingHeader(200.0f, 78.0f, true);
    FrameCollapsingHeader(200.0f, 78.0f, false);
    // Hovering while it is open, then a second click closes it.
    FrameCollapsingHeader(200.0f, 78.0f, false);
    FrameCollapsingHeader(200.0f, 78.0f, true);
    FrameCollapsingHeader(200.0f, 78.0f, false);
    // A click on the arrow toggles on press instead.
    FrameCollapsingHeader(72.0f, 95.0f, false);
    FrameCollapsingHeader(72.0f, 95.0f, true);
    FrameCollapsingHeader(72.0f, 95.0f, false);
}

// tree_node: unframed tree nodes. A click on the arrow opens on press and a click on the label on
// release, children move right by IndentSpacing, and the hover highlight spans the work area even
// though the row only reacts where the text is.
static void FrameTreeNode(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("T", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGuiWindow* w = ImGui::GetCurrentWindow();
    if (ImGui::TreeNode("Root"))
    {
        ImGui::Dummy(ImVec2(10.0f, 8.0f));
        if (ImGui::TreeNode("Leaf"))
        {
            ImGui::Dummy(ImVec2(10.0f, 8.0f));
            ImGui::TreePop();
        }
        ImGui::TreePop();
    }
    ImGui::Dummy(ImVec2(10.0f, 8.0f));
    printf("frame done\n");
    DumpDrawListBody(*w->DrawList);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpTreeNode()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tree_node\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[7] = { 'R', 'o', 't', 'L', 'e', 'a', 'f' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    ImGui::NewFrame();
    WriteWindowIconsFixture(font, bake_order, 7);
    ImGui::EndFrame();

    // The root node sits at the top of the window, spanning y 68 to 81.
    for (int i = 0; i < 3; i++)
        FrameTreeNode(0.0f, 0.0f, false);
    // A click on the label opens it on release, which shows the indented leaf underneath.
    FrameTreeNode(100.0f, 74.0f, false);
    FrameTreeNode(100.0f, 74.0f, true);
    FrameTreeNode(100.0f, 74.0f, false);
    // Hovering the child, then opening it with its arrow.
    FrameTreeNode(100.0f, 90.0f, false);
    FrameTreeNode(95.0f, 90.0f, false);
    FrameTreeNode(95.0f, 90.0f, true);
    FrameTreeNode(95.0f, 90.0f, false);
    // A click on the label of the root closes it again.
    FrameTreeNode(100.0f, 74.0f, false);
    FrameTreeNode(100.0f, 74.0f, true);
    FrameTreeNode(100.0f, 74.0f, false);
}

static void FrameChildRegionWheelDown(float mx, float my, float wheel, bool down);

// child_region: BeginChild gives a region its own layout, padding, border and draw list. The child's
// list is submitted right after its host's, so the two are dumped in that order.
static void FrameChildRegionWheel(float mx, float my, float wheel)
{
    FrameChildRegionWheelDown(mx, my, wheel, false);
}

static void FrameChildRegionWheelDown(float mx, float my, float wheel, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    io.MouseWheel = wheel;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("P", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGui::Button("AA");
    ImGui::BeginChild("Inside", ImVec2(150.0f, 60.0f), ImGuiChildFlags_Borders);
    ImGui::Button("BB");
    ImGui::Button("CC");
    ImGui::Button("DD");
    ImGui::EndChild();
    ImGui::Button("EE");
    ImGui::End();
    io.MouseWheel = 0.0f;
}

static void FrameChildRegion()
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(0.0f, 0.0f);
    io.MouseDown[0] = false;
    io.MouseWheel = 0.0f;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("P", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGui::Button("AA");
    ImGui::BeginChild("Inside", ImVec2(150.0f, 60.0f), ImGuiChildFlags_Borders);
    ImGui::Button("BB");
    ImGui::Button("CC");
    ImGui::Button("DD");
    ImGui::EndChild();
    ImGui::Button("EE");
    ImGui::End();
}

static void DumpChildRegion()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario child_region\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    const char bake_order[5] = { 'A', 'B', 'C', 'D', 'E' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 5);

    for (int f = 0; f < 4; f++)
    {
        FrameChildRegion();
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }

    // The wheel over the region scrolls the region, not the window hosting it.
    const float wheel_steps[4] = { -1.0f, -1.0f, 0.0f, 1.0f };
    for (int f = 4; f < 8; f++)
    {
        FrameChildRegionWheel(140.0f, 120.0f, wheel_steps[f - 4]);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }

    // Grabbing the region's own scrollbar and dragging it down.
    const float bar_pos[5][3] = {
        { 210.0f, 100.0f, 0.0f },
        { 210.0f, 100.0f, 1.0f },
        { 210.0f, 120.0f, 1.0f },
        { 210.0f, 130.0f, 1.0f },
        { 210.0f, 130.0f, 0.0f },
    };
    for (int f = 8; f < 13; f++)
    {
        FrameChildRegionWheelDown(bar_pos[f - 8][0], bar_pos[f - 8][1], 0.0f, bar_pos[f - 8][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// selectable: a row fills the available width and highlights when hovered or selected.
static int g_selectable_selected = 0;

static void FrameSelectable(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(200.0f, 160.0f));
    ImGui::Begin("S", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    static const char* items[3] = { "One", "Two", "Three" };
    for (int n = 0; n < 3; n++)
        if (ImGui::Selectable(items[n], g_selectable_selected == n))
            g_selectable_selected = n;
    printf("frame selected %d\n", g_selectable_selected);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpSelectable()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario selectable\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every row: the reference bakes on demand, so a missing one changes the picture.
    const char bake_order[8] = { 'O', 'n', 'e', 'T', 'w', 'o', 'h', 'r' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 8);

    const float steps[8][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 100.0f, 75.0f, 0.0f },
        { 100.0f, 75.0f, 1.0f },
        { 100.0f, 75.0f, 0.0f },
        { 100.0f, 95.0f, 0.0f },
        { 100.0f, 95.0f, 1.0f },
        { 100.0f, 95.0f, 0.0f },
    };
    for (int f = 0; f < 8; f++)
    {
        FrameSelectable(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            printf("list %d S\n", i);
            DumpDrawListBody(*dd->CmdLists[i]);
        }
    }
}
// popup: a button opens a popup, the popup draws with its own background in a list of its own, and a
// click outside closes it on the following frame.
static void FramePopup(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("P", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    if (ImGui::Button("Open"))
        ImGui::OpenPopup("##p");
    ImGui::SetNextWindowPos(ImVec2(120.0f, 110.0f));
    if (ImGui::BeginPopup("##p"))
    {
        static const char* items[3] = { "One", "Two", "Three" };
        for (int n = 0; n < 3; n++)
            ImGui::Selectable(items[n], false);
        ImGui::EndPopup();
    }
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpPopup()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario popup\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every row, since the reference bakes on demand.
    const char bake_order[10] = { 'O', 'p', 'e', 'n', 'T', 'w', 'o', 'h', 'r', 'i' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 10);

    // The button is at (68,68) to (97,87): press and release it, then click outside the popup.
    const float steps[10][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 80.0f, 75.0f, 0.0f },
        { 80.0f, 75.0f, 1.0f },
        { 80.0f, 75.0f, 0.0f },
        { 150.0f, 120.0f, 0.0f },
        { 150.0f, 120.0f, 1.0f },
        { 400.0f, 300.0f, 1.0f },
        { 400.0f, 300.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
    };
    for (int f = 0; f < 10; f++)
    {
        FramePopup(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// popup_click: a click on a row inside a popup. The row must report the press on release, the state
// must clear on the next frame, and choosing a row closes the popup one frame later.
static int g_popup_row = -1;

static void FramePopupClick(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("P", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    if (ImGui::Button("Open"))
        ImGui::OpenPopup("##p");
    ImGui::SetNextWindowPos(ImVec2(120.0f, 110.0f));
    if (ImGui::BeginPopup("##p"))
    {
        static const char* items[3] = { "One", "Two", "Three" };
        for (int n = 0; n < 3; n++)
            if (ImGui::Selectable(items[n], n == g_popup_row))
                g_popup_row = n;
        ImGui::EndPopup();
    }
    printf("frame row %d\n", g_popup_row);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpPopupClick()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario popup_click\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every row, since the reference bakes on demand.
    const char bake_order[10] = { 'O', 'p', 'e', 'n', 'T', 'w', 'o', 'h', 'r', 'i' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 10);

    // The popup sits at (120,110); its rows run from y 120 down. Frame 2 presses Open, 3 releases it.
    const float steps[12][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 80.0f, 75.0f, 1.0f },
        { 80.0f, 75.0f, 0.0f },
        { 150.0f, 135.0f, 0.0f },
        { 150.0f, 135.0f, 1.0f },
        { 150.0f, 135.0f, 0.0f },
        { 150.0f, 135.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
    };
    for (int f = 0; f < 12; f++)
    {
        FramePopupClick(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// combo: the box shows its preview and its popup is a window of its own under it, holding the rows.
static int g_combo_current = 0;

static void FrameCombo(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    static const char* items[3] = { "One", "Two", "Three" };
    ImGui::SetNextItemWidth(150.0f);
    if (ImGui::BeginCombo("##combo", items[g_combo_current], ImGuiComboFlags_None))
    {
        for (int n = 0; n < 3; n++)
            if (ImGui::Selectable(items[n], g_combo_current == n))
                g_combo_current = n;
        ImGui::EndCombo();
    }
    printf("frame current %d\n", g_combo_current);
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpCombo()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario combo\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every row, since the reference bakes on demand.
    const char bake_order[8] = { 'O', 'n', 'e', 'T', 'w', 'o', 'h', 'r' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    // The arrow is a stroke, so the fixture carries the line rows too.
    ImGui::NewFrame();
    WriteWindowIconsFixture(font, bake_order, 8);
    ImGui::EndFrame();

    // The box is at (68,68) to (218,87): press it, hover a row, choose it, then click outside.
    const float steps[11][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 140.0f, 75.0f, 0.0f },
        { 140.0f, 75.0f, 1.0f },
        { 140.0f, 75.0f, 0.0f },
        { 140.0f, 108.0f, 0.0f },
        { 140.0f, 108.0f, 1.0f },
        { 140.0f, 108.0f, 0.0f },
        { 140.0f, 135.0f, 0.0f },
        { 5.0f, 5.0f, 1.0f },
        { 5.0f, 5.0f, 0.0f },
    };
    for (int f = 0; f < 11; f++)
    {
        FrameCombo(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *ImGui::GetCurrentContext();
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// tooltip: a tooltip is a window at the mouse that takes neither the mouse nor the keyboard, so the
// item under it keeps hovering while it shows.
static void FrameTooltip(float mx, float my)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = false;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    ImGui::Button("Hover");
    if (ImGui::IsItemHovered())
    {
        ImGui::SetTooltip("Tip");
    }
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpTooltip()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario tooltip\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of the button and of the tip.
    const char bake_order[8] = { 'H', 'o', 'v', 'e', 'r', 'T', 'i', 'p' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 8);

    // The button is at (68,68) to (105,87): mouse away, on it, then away again.
    const float moves[6][2] = {
        { 0.0f, 0.0f },
        { 0.0f, 0.0f },
        { 80.0f, 75.0f },
        { 80.0f, 75.0f },
        { 400.0f, 400.0f },
        { 400.0f, 400.0f },
    };
    for (int f = 0; f < 6; f++)
    {
        FrameTooltip(moves[f][0], moves[f][1]);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// menu_bar: a window with a menu bar, whose two entries are laid out in a row and take their height
// out of the content area below.
static void FrameMenuBar(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar |
        ImGuiWindowFlags_MenuBar);
    if (ImGui::BeginMenuBar())
    {
        ImGui::MenuItem("File");
        ImGui::MenuItem("Edit");
        ImGui::EndMenuBar();
    }
    ImGui::Button("x");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpMenuBar()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario menu_bar\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every string, since the reference bakes on demand.
    const char bake_order[8] = { 'F', 'i', 'l', 'e', 'E', 'd', 't', 'x' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 8);

    // The bar runs across the top: idle, hover the first entry, hover the second, come back.
    const float steps[5][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 75.0f, 68.0f, 0.0f },
        { 130.0f, 68.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
    };
    for (int f = 0; f < 5; f++)
    {
        FrameMenuBar(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
}
// submenu: a menu-bar entry opens a child menu of its own. The child menu is a window that is
// measured on its first frame and only draws from the second; its rows are menu items laid out in
// columns whose widths are accumulated during a frame and locked from the previous one.
static void FrameSubmenu(float mx, float my, bool down)
{
    ImGuiIO& io = ImGui::GetIO();
    io.DeltaTime = 0.1f;
    io.MousePos = ImVec2(mx, my);
    io.MouseDown[0] = down;
    ImGui::NewFrame();
    ImGui::SetNextWindowPos(ImVec2(60.0f, 60.0f));
    ImGui::SetNextWindowSize(ImVec2(300.0f, 200.0f));
    ImGui::Begin("W", NULL, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar |
        ImGuiWindowFlags_MenuBar);
    if (ImGui::BeginMenuBar())
    {
        if (ImGui::BeginMenu("File"))
        {
            ImGui::MenuItem("New", "Ctrl+N");
            ImGui::MenuItem("Open");
            ImGui::EndMenu();
        }
        ImGui::MenuItem("Edit");
        ImGui::EndMenuBar();
    }
    ImGui::Button("x");
    ImGui::End();
    ImGui::EndFrame();
}

static void DumpSubmenu()
{
    printf("header imgui %s %d\n", IMGUI_VERSION, IMGUI_VERSION_NUM);
    printf("scenario submenu\n");
    ImGuiIO& io = ImGui::GetIO();
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;
    if (!io.Fonts->Build() || io.Fonts->Fonts.Size == 0)
    {
        printf("font build failed\n");
        return;
    }
    ImFont* font = io.Fonts->Fonts[0];
    ImFontBaked* baked = font->GetFontBaked(13.0f);
    // Every letter of every string, since the reference bakes on demand.
    const char bake_order[16] = { 'F', 'i', 'l', 'e', 'N', 'w', 'C', 't', 'r', '+', 'O', 'p', 'n', 'E', 'd', 'x' };
    for (char c : bake_order)
        baked->FindGlyph((ImWchar)c);
    WriteGlyphFixture(font, bake_order, 16);

    // The bar runs across the top at y 60..79. Frame 2 presses "File", 3 releases it, 4 hovers the
    // first row of the menu, 5 presses it and 6 releases it so the choice lands and the menu closes.
    // Frame 9 then hovers "Edit", a plain menu item, whose box is only as tall as its label.
    const float steps[11][3] = {
        { 0.0f, 0.0f, 0.0f },
        { 75.0f, 68.0f, 0.0f },
        { 75.0f, 68.0f, 1.0f },
        { 75.0f, 68.0f, 0.0f },
        { 100.0f, 95.0f, 0.0f },
        { 100.0f, 95.0f, 1.0f },
        { 100.0f, 95.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
        { 130.0f, 62.0f, 0.0f },
        { 0.0f, 0.0f, 0.0f },
    };
    for (int f = 0; f < 11; f++)
    {
        FrameSubmenu(steps[f][0], steps[f][1], steps[f][2] != 0.0f);
        ImGui::Render();
        ImDrawData* dd = ImGui::GetDrawData();
        ImGuiContext& g = *GImGui;
        printf("frame %d lists %d\n", f, dd->CmdListsCount);
        for (int i = 0; i < dd->CmdListsCount; i++)
        {
            const ImDrawList* list = dd->CmdLists[i];
            const char* name = "?";
            for (int w = 0; w < g.Windows.Size; w++)
                if (g.Windows[w]->DrawList == list)
                    name = g.Windows[w]->Name;
            printf("list %d %s\n", i, name);
            DumpDrawListBody(*list);
        }
    }
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
static void SceneRectStroke(ImDrawList& list)
{
    // `AddRect` without rounding: whole coordinates take the pixel-aligned fast path (one quad per
    // side), a fractional rectangle takes the closed inside-stroked polyline.
    list.AddRect(ImVec2(10.0f, 10.0f), ImVec2(60.0f, 50.0f), COL, 0.0f, 1.0f);
    list.AddRect(ImVec2(80.0f, 10.0f), ImVec2(140.0f, 50.0f), COL, 0.0f, 3.0f);
    list.AddRect(ImVec2(150.5f, 10.25f), ImVec2(200.5f, 50.75f), COL, 0.0f, 1.0f);
}

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

    if (strcmp(name, "input_text_mouse_undo") == 0)
    {
        DumpInputTextMouseUndo();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "input_text_clipboard") == 0)
    {
        DumpInputTextClipboard();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "input_text_edit") == 0)
    {
        DumpInputTextEdit();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "input_text") == 0)
    {
        DumpInputText();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "text_clip") == 0)
    {
        DumpTextClip();
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


    if (strcmp(name, "window_bg_alpha") == 0)
    {
        DumpWindowBgAlpha();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "cursor_pos") == 0)
    {
        DumpCursorPos();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_set_focus") == 0)
    {
        DumpWindowsSetFocus();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_set_collapsed") == 0)
    {
        DumpWindowsSetCollapsed();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "invisible_button") == 0)
    {
        DumpInvisibleButton();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "indent") == 0)
    {
        DumpIndent();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "new_line") == 0)
    {
        DumpNewLine();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "spacing") == 0)
    {
        DumpSpacing();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "separator_text_empty") == 0)
    {
        DumpSeparatorTextEmpty();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "bullet_text") == 0)
    {
        DumpBulletText();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "label_text") == 0)
    {
        DumpLabelText();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "text_colors") == 0)
    {
        DumpTextColors();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "progress_bar") == 0)
    {
        DumpProgressBar();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "bullet") == 0)
    {
        DumpBullet();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "disabled") == 0)
    {
        DumpDisabled();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "menu_bar_separator") == 0)
    {
        DumpMenuBarSeparator();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "menu_bar") == 0)
    {
        DumpMenuBar();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "submenu") == 0)
    {
        DumpSubmenu();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tooltip") == 0)
    {
        DumpTooltip();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "combo") == 0)
    {
        DumpCombo();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "popup_click") == 0)
    {
        DumpPopupClick();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "popup") == 0)
    {
        DumpPopup();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "selectable") == 0)
    {
        DumpSelectable();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "child_region") == 0)
    {
        DumpChildRegion();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tree_node") == 0)
    {
        DumpTreeNode();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "collapsing_header") == 0)
    {
        DumpCollapsingHeader();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_resize_autofit") == 0)
    {
        DumpWindowResizeAutofit();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_resize_cursor") == 0)
    {
        DumpWindowResizeCursor();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_resize_edges") == 0)
    {
        DumpWindowResizeEdges();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_resize_drag") == 0)
    {
        DumpWindowResizeDrag();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_resize_grips") == 0)
    {
        DumpWindowResizeGrips();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_min_size") == 0)
    {
        DumpWindowMinSize();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_scrollbar_page") == 0)
    {
        DumpWindowScrollbarPage();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "window_hscrollbar") == 0)
    {
        DumpWindowHScrollbar();
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

    if (strcmp(name, "windows_focus") == 0)
    {
        DumpWindowsFocus();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "separator_text") == 0)
    {
        DumpSeparatorText();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "separator") == 0)
    {
        DumpSeparator();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_set_size") == 0)
    {
        DumpWindowsSetSize();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_set_pos") == 0)
    {
        DumpWindowsSetPos();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "windows_collapse_api") == 0)
    {
        DumpWindowsCollapseApi();
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

    if (strcmp(name, "io_text_events") == 0)
    {
        DumpIoText();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "cmd_split") == 0)
    {
        DumpCmdSplit();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "aa_circles") == 0)
    {
        DumpAaCircles();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_basic") == 0)
    {
        DumpTablesBasic();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_headers") == 0)
    {
        DumpTablesHeaders();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_rowbg") == 0)
    {
        DumpTablesRowBg();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_cellbg") == 0)
    {
        DumpTablesCellBg();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_borders") == 0)
    {
        DumpTablesBorders();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_borders_outer") == 0)
    {
        DumpTablesBordersOuter();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_stretch_weights") == 0)
    {
        DumpTablesStretchWeights();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_stretch") == 0)
    {
        DumpTablesStretch();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_set_column") == 0)
    {
        DumpTablesSetColumn();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_after_end") == 0)
    {
        DumpTablesAfterEnd();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_column_sorted_flag") == 0)
    {
        DumpTablesColumnSortedFlag();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_sort_tristate") == 0)
    {
        DumpTablesSortTristate();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_scroll_child") == 0)
    {
        DumpTablesScrollChild();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_sort_multi") == 0)
    {
        DumpTablesSortMulti();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_reorder") == 0)
    {
        DumpTablesReorder();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_sort_click") == 0)
    {
        DumpTablesSortClick();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_sort_arrow") == 0)
    {
        DumpTablesSortArrow();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_sort_specs") == 0)
    {
        DumpTablesSortSpecs();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_column_hovered_flag") == 0)
    {
        DumpTablesColumnHoveredFlag();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_cell_bg_twice") == 0)
    {
        DumpTablesCellBgTwice();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_column_bounds") == 0)
    {
        DumpTablesColumnBounds();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_row_bg_color") == 0)
    {
        DumpTablesRowBgColor();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_stretch_prop") == 0)
    {
        DumpTablesStretchProp();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_hovered") == 0)
    {
        DumpTablesHovered();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_header_active") == 0)
    {
        DumpTablesHeaderActive();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_header_hover") == 0)
    {
        DumpTablesHeaderHover();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_next_column") == 0)
    {
        DumpTablesNextColumn();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_rowbg_headers") == 0)
    {
        DumpTablesRowBgHeaders();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_column_flags") == 0)
    {
        DumpTablesColumnFlags();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_header_inrow") == 0)
    {
        DumpTablesHeaderInRow();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_auto_width") == 0)
    {
        DumpTablesAutoWidth();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_clip") == 0)
    {
        DumpTablesClip();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_header_id") == 0)
    {
        DumpTablesHeaderId();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_names") == 0)
    {
        DumpTablesNames();
        ImGui::DestroyContext();
        return 0;
    }

    if (strcmp(name, "tables_row_height") == 0)
    {
        DumpTablesRowHeight();
        ImGui::DestroyContext();
        return 0;
    }

    void (*scene)(ImDrawList&) = NULL;
    if (strcmp(name, "prim_rect_uv") == 0) scene = ScenePrimRectUV;
    else if (strcmp(name, "prim_rect") == 0) scene = ScenePrimRect;
    else if (strcmp(name, "rect_filled") == 0) scene = SceneRectFilled;
    else if (strcmp(name, "line_h") == 0) scene = SceneLineH;
    else if (strcmp(name, "rect_stroke") == 0) scene = SceneRectStroke;
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