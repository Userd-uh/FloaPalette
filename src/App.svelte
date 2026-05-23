<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import defaultData from "../data/commands.json";
  import type { AppData, Category, CommandItem } from "./lib/types";

  type TauriWindow = ReturnType<typeof getCurrentWindow>;
  type EditorMode = "command" | "category" | null;

  let appData: AppData = {
    appName: "FloaPalette",
    version: 1,
    categories: []
  };

  let selectedCategoryId = "";
  let selectedItemId = "";
  let search = "";
  let selectedCategory: Category | null = null;
  let selectedItem: CommandItem | null = null;
  let items: CommandItem[] = [];
  let editorMode: EditorMode = null;
  let status = "起動中";
  let draftMode: "edit" | "add" = "edit";
  let draftItem: CommandItem | null = null;
  let draftCategoryLabel = "";
  let draftCategoryColor = "#6bc7ff";
  let dataFilePath = "";
  let appWindow: TauriWindow | null = null;
  let categoryNameInput: HTMLInputElement | null = null;

  const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  const cloneData = (data: AppData): AppData => JSON.parse(JSON.stringify(data));

  const emptyItem = (): CommandItem => ({
    id: crypto.randomUUID(),
    title: "",
    command: "",
    description: "",
    favorite: false,
    autoEnter: false,
    inputMode: "text"
  });

  const looksLikeShortcut = (value: string) =>
    /(^|[\s/])(ctrl|control|shift|alt|win|windows|cmd|command|meta)\s*\+/i.test(value.trim());

  const itemInputMode = (item: CommandItem) =>
    item.inputMode ?? (looksLikeShortcut(item.command) ? "shortcut" : "text");

  const normalizeData = (data: AppData): AppData => ({
    ...data,
    categories: data.categories.map((category) => ({
      ...category,
      items: category.items.map((item) => ({
        ...item,
        inputMode: itemInputMode(item)
      }))
    }))
  });

  const resetSelection = () => {
    selectedCategoryId = appData.categories[0]?.id ?? "";
    selectedItemId = appData.categories[0]?.items[0]?.id ?? "";
    search = "";
    editorMode = null;
    draftItem = null;
    refreshSelection();
  };

  const save = async () => {
    if (!isTauri()) {
      status = "ブラウザ表示中 / 保存はTauri起動時のみ";
      return;
    }
    await invoke("save_app_data", { data: appData });
    dataFilePath = await invoke<string>("get_data_file_path");
    status = "保存済み";
  };

  const load = async () => {
    try {
      appData = isTauri()
        ? await invoke<AppData>("load_app_data")
        : cloneData(defaultData as AppData);
      appData = normalizeData(appData);
      dataFilePath = isTauri()
        ? await invoke<string>("get_data_file_path")
        : "data/commands.json (preview)";
    } catch (e) {
      appData = normalizeData(cloneData(defaultData as AppData));
      dataFilePath = "data/commands.json";
      status = `JSON読み込みフォールバック: ${String(e)}`;
    }
    if (!selectedCategoryId && appData.categories.length > 0) {
      selectedCategoryId = appData.categories[0].id;
    }
  };

  const refreshSelection = () => {
    selectedCategory = appData.categories.find((c) => c.id === selectedCategoryId) ?? null;
    if (!selectedCategory) {
      selectedItem = null;
      items = [];
      return;
    }

    const q = search.trim().toLowerCase();
    items = q
      ? selectedCategory.items.filter((i) =>
          `${i.title} ${i.command} ${i.description}`.toLowerCase().includes(q)
        )
      : selectedCategory.items;

    if (selectedItemId && !selectedCategory.items.some((i) => i.id === selectedItemId)) {
      selectedItemId = selectedCategory.items[0]?.id ?? "";
    }

    selectedItem =
      selectedCategory.items.find((i) => i.id === selectedItemId) ??
      selectedCategory.items[0] ??
      null;
  };

  const chooseCategory = (id: string) => {
    selectedCategoryId = id;
    search = "";
    const cat = appData.categories.find((c) => c.id === id);
    selectedItemId = cat?.items[0]?.id ?? "";
    editorMode = null;
    draftItem = null;
    refreshSelection();
  };

  const startWindowDrag = async (event: MouseEvent) => {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, input, textarea, select")) return;
    await appWindow?.startDragging();
  };

  const sendItem = async (item: CommandItem) => {
    selectedItemId = item.id;
    refreshSelection();
    if (!item.command.trim()) {
      status = "送信するコマンドがありません";
      return;
    }
    status = "送信中";
    try {
      if (isTauri()) {
        if (itemInputMode(item) === "shortcut") {
          await invoke("send_shortcut_to_last_window", {
            shortcut: item.command
          });
          status = "ショートカット実行済み";
        } else {
          await invoke("send_command_to_last_window", {
            command: item.command,
            autoEnter: item.autoEnter
          });
          status = item.autoEnter ? "送信済み / Enter付き" : "送信済み";
        }
      } else {
        await navigator.clipboard.writeText(item.command);
        status = "ブラウザ表示中 / クリップボードへコピー";
      }
    } catch (e) {
      status = `送信失敗: ${String(e)}`;
    }
  };

  const minimizeWindow = async () => {
    await appWindow?.minimize();
  };

  const closeWindow = async () => {
    if (isTauri()) {
      await invoke("exit_app");
    }
  };

  const openCommandEditor = (item: CommandItem, mode: "edit" | "add" = "edit") => {
    if (!selectedCategory) return;
    draftMode = mode;
    selectedItemId = item.id;
    selectedItem = mode === "edit" ? item : null;
    draftItem = { ...item };
    editorMode = "command";
  };

  const openCategoryEditor = async () => {
    if (!selectedCategory) return;
    draftCategoryLabel = selectedCategory.label;
    draftCategoryColor = selectedCategory.color;
    draftItem = null;
    editorMode = "category";
    await tick();
    categoryNameInput?.focus();
    categoryNameInput?.select();
  };

  const addCategory = async () => {
    const cat: Category = {
      id: crypto.randomUUID(),
      label: "NEW",
      color: "#6bc7ff",
      items: []
    };
    appData = { ...appData, categories: [...appData.categories, cat] };
    selectedCategoryId = cat.id;
    selectedItemId = "";
    refreshSelection();
    await save();
    await openCategoryEditor();
  };

  const addItem = () => {
    if (!selectedCategory) return;
    openCommandEditor(emptyItem(), "add");
  };

  const toggleEditor = () => {
    if (editorMode === "command") {
      editorMode = null;
      draftItem = null;
      return;
    }
    if (selectedItem) {
      openCommandEditor(selectedItem);
    } else if (selectedCategory) {
      openCommandEditor(emptyItem(), "add");
    }
  };

  const applyDraft = async () => {
    if (!selectedCategory || !draftItem) return;
    if (!draftItem.title.trim() || !draftItem.command.trim()) {
      status = "タイトルとコマンドを入力してください";
      return;
    }
    if (itemInputMode(draftItem) === "shortcut") {
      draftItem.autoEnter = false;
    }

    if (draftMode === "add") {
      selectedCategory.items = [...selectedCategory.items, { ...draftItem }];
      selectedItemId = draftItem.id;
    } else {
      selectedCategory.items = selectedCategory.items.map((item) =>
        item.id === draftItem?.id ? { ...draftItem } : item
      );
      selectedItemId = draftItem.id;
    }

    appData = { ...appData, categories: [...appData.categories] };
    refreshSelection();
    await save();
    editorMode = null;
    draftItem = null;
  };

  const applyCategoryDraft = async () => {
    if (!selectedCategory) return;
    const nextLabel = draftCategoryLabel.trim();
    if (!nextLabel) {
      status = "カテゴリ名を入力してください";
      return;
    }
    selectedCategory.label = nextLabel;
    selectedCategory.color = draftCategoryColor.trim() || selectedCategory.color;
    appData = { ...appData, categories: [...appData.categories] };
    refreshSelection();
    await save();
    editorMode = null;
  };

  const deleteItem = async () => {
    if (!selectedCategory || !selectedItem) return;
    selectedCategory.items = selectedCategory.items.filter((i) => i.id !== selectedItem?.id);
    selectedItemId = selectedCategory.items[0]?.id ?? "";
    appData = { ...appData, categories: [...appData.categories] };
    editorMode = null;
    draftItem = null;
    refreshSelection();
    await save();
  };

  const deleteCategory = async () => {
    if (!selectedCategory) return;
    appData = {
      ...appData,
      categories: appData.categories.filter((c) => c.id !== selectedCategory?.id)
    };
    selectedCategoryId = appData.categories[0]?.id ?? "";
    selectedItemId = "";
    editorMode = null;
    draftItem = null;
    refreshSelection();
    await save();
  };

  const chooseDataFile = async () => {
    if (!isTauri()) {
      status = "ファイル選択はTauri起動時のみ使えます";
      return;
    }

    const selected = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }]
    });
    if (!selected || Array.isArray(selected)) return;

    try {
      const loaded = await invoke<AppData>("load_app_data_from_file", { path: selected });
      await invoke("set_data_file_path", { path: selected });
      appData = normalizeData(loaded);
      dataFilePath = selected;
      resetSelection();
      status = "データファイルを読み込みました";
    } catch (e) {
      status = `データファイルを読み込めません: ${String(e)}`;
    }
  };

  onMount(async () => {
    if (isTauri()) {
      appWindow = getCurrentWindow();
      window.addEventListener("blur", () => {
        invoke("record_foreground_target_delayed").catch(() => undefined);
      });
    }
    await load();
    refreshSelection();
    await appWindow?.setAlwaysOnTop(true);
    await appWindow?.setDecorations(false);
    await appWindow?.setResizable(true);
    await appWindow?.setSkipTaskbar(false);
    status = "常駐準備完了";
  });

  $: refreshSelection();
</script>

<div class:editor-open={editorMode !== null} class="shell">
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
  <header class="topbar" on:mousedown={startWindowDrag}>
    <div class="brand">
      <div class="logo"></div>
      <div>
        <div class="title">{appData.appName}</div>
        <div class="subtitle">{status}</div>
      </div>
    </div>

    <div class="actions">
      <input class="search" bind:value={search} placeholder="検索..." />
      <button class="ghost" on:click={addCategory}>カテゴリ追加</button>
      <button class="ghost" on:click={addItem} disabled={!selectedCategory}>コマンド追加</button>
      <button class:selected={editorMode === "command"} class="ghost" on:click={toggleEditor}>
        コマンド編集
      </button>
      <div class="window-controls">
        <button class="window-button" title="最小化" aria-label="最小化" on:click={minimizeWindow}>
          -
        </button>
        <button class="window-button close" title="閉じる" aria-label="閉じる" on:click={closeWindow}>
          X
        </button>
      </div>
    </div>
  </header>

  <section class="datafile-bar">
    <div class="datafile-text">
      <span>現在のデータファイル</span>
      <strong title={dataFilePath}>{dataFilePath || "未設定"}</strong>
    </div>
    <button class="ghost" on:click={chooseDataFile}>ファイルを選択</button>
  </section>

  <main class="main">
    <section class="leftpane">
      <div class="category-strip">
        {#each appData.categories as cat}
          <button
            class:selected={cat.id === selectedCategoryId}
            class="category"
            style={`--accent:${cat.color}`}
            on:click={() => chooseCategory(cat.id)}
          >
            {cat.label}
          </button>
        {/each}
      </div>

      <div class="listhead">
        <div>
          <div class="listtitle">{selectedCategory?.label ?? "カテゴリ未選択"}</div>
          <div class="listhint">{items.length} コマンド</div>
        </div>
        <button class="mini" on:click={openCategoryEditor} disabled={!selectedCategory}>
          カテゴリ編集
        </button>
      </div>

      <div class="list">
        {#if items.length === 0}
          <div class="emptylist">
            {#if selectedCategory && selectedCategory.items.length > 0}
              検索条件に一致するコマンドがありません。
            {:else if selectedCategory}
              このカテゴリにはまだコマンドがありません。
            {:else}
              カテゴリがありません。
            {/if}
          </div>
        {/if}
        {#each items as item}
          <div
            class:selected={item.id === selectedItemId}
            class="itemcard"
            role="button"
            tabindex="0"
            on:click={() => sendItem(item)}
            on:keydown={(event) => {
              if (event.key === "Enter" || event.key === " ") sendItem(item);
            }}
          >
            <div class="cardtop">
              <div>
                <div class="fieldlabel">タイトル</div>
                <div class="itemtitle">{item.title}</div>
              </div>
              {#if item.favorite}
                <span class="fav">★</span>
              {/if}
            </div>
            <div class="cardfield">
              <div class="fieldlabel">説明</div>
              <div class="desc">{item.description}</div>
            </div>
            <div class="cardfield commandfield">
              <div class="fieldlabel">
                {itemInputMode(item) === "shortcut" ? "ショートカット" : "入力"}
              </div>
              <div class="command">{item.command}</div>
            </div>
            <div class="cardactions">
              <button class="mini" on:click|stopPropagation={() => openCommandEditor(item)}>
                編集
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>

    {#if editorMode !== null}
      <aside class="rightpane">
        {#if editorMode === "command" && selectedCategory && draftItem}
          <div class="editor">
            <div class="editorhead">
              <div>
                <div class="editlabel">{draftMode === "add" ? "コマンド追加" : "コマンド編集"}</div>
                <div class="edithint">編集内容は適用ボタンで保存します</div>
              </div>
              <div class="editorbuttons">
                {#if draftMode === "edit" && selectedItem}
                  <button class="ghost danger" on:click={deleteItem}>コマンド削除</button>
                {/if}
              </div>
            </div>

            <label>
              <span>タイトル</span>
              <input bind:value={draftItem.title} />
            </label>

            <label>
              <span>コマンド</span>
              <textarea rows="8" bind:value={draftItem.command}></textarea>
            </label>

            <label>
              <span>入力方式</span>
              <select bind:value={draftItem.inputMode}>
                <option value="text">コマンド文字入力</option>
                <option value="shortcut">ショートカット実行</option>
              </select>
            </label>

            <label>
              <span>説明</span>
              <input bind:value={draftItem.description} />
            </label>

            <label class="checkrow">
              <input type="checkbox" bind:checked={draftItem.favorite} />
              <span>お気に入り</span>
            </label>

            <label class="checkrow">
              <input
                type="checkbox"
                bind:checked={draftItem.autoEnter}
                disabled={itemInputMode(draftItem) === "shortcut"}
              />
              <span>送信後にEnter</span>
            </label>

            <button class="primary" on:click={applyDraft}>適用</button>
          </div>
        {:else if editorMode === "category" && selectedCategory}
          <div class="editor">
            <div class="editorhead">
              <div>
                <div class="editlabel">カテゴリ編集</div>
                <div class="edithint">カテゴリ名と色を編集します</div>
              </div>
              <div class="editorbuttons">
                <button class="ghost danger" on:click={deleteCategory}>カテゴリ削除</button>
              </div>
            </div>

            <label>
              <span>カテゴリ名</span>
              <input bind:this={categoryNameInput} bind:value={draftCategoryLabel} />
            </label>

            <label>
              <span>カテゴリ色</span>
              <div class="colorrow">
                <input class="colorinput" type="color" bind:value={draftCategoryColor} />
                <input bind:value={draftCategoryColor} />
              </div>
            </label>

            <button class="primary" on:click={applyCategoryDraft}>適用</button>
          </div>
        {:else}
          <div class="empty">編集対象を選択してください。</div>
        {/if}
      </aside>
    {/if}
  </main>
</div>
