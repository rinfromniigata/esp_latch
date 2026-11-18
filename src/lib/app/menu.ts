// アプリメニュー。文言は辞書から作り、ロケールが変わったら作り直す（メインウィンドウが受け持つ）
import { isTauri } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { m } from '#lib/i18n/index.js';
import { openAppWindow } from './windows';

const separator = () => PredefinedMenuItem.new({ item: 'Separator' });

/** アプリメニューを現在のロケールで組み立てて差し替える */
export async function installAppMenu(): Promise<void> {
	if (!isTauri()) return;

	const appMenu = await Submenu.new({
		text: m['app.name'](),
		items: [
			await PredefinedMenuItem.new({ item: { About: null }, text: m['menu.app.about']() }),
			await separator(),
			await MenuItem.new({
				id: 'settings',
				text: m['menu.app.settings'](),
				accelerator: 'CmdOrCtrl+,',
				action: () => void openAppWindow('settings')
			}),
			await separator(),
			await PredefinedMenuItem.new({ item: 'Services', text: m['menu.app.services']() }),
			await separator(),
			await PredefinedMenuItem.new({ item: 'Hide', text: m['menu.app.hide']() }),
			await PredefinedMenuItem.new({ item: 'HideOthers', text: m['menu.app.hide_others']() }),
			await PredefinedMenuItem.new({ item: 'ShowAll', text: m['menu.app.show_all']() }),
			await separator(),
			await PredefinedMenuItem.new({ item: 'Quit', text: m['menu.app.quit']() })
		]
	});

	// テキストの入力・コピーを、WebViewの中でも⌘C・⌘Vなどで使えるようにする
	const editMenu = await Submenu.new({
		text: m['menu.edit.title'](),
		items: [
			await PredefinedMenuItem.new({ item: 'Undo', text: m['menu.edit.undo']() }),
			await PredefinedMenuItem.new({ item: 'Redo', text: m['menu.edit.redo']() }),
			await separator(),
			await PredefinedMenuItem.new({ item: 'Cut', text: m['menu.edit.cut']() }),
			await PredefinedMenuItem.new({ item: 'Copy', text: m['menu.edit.copy']() }),
			await PredefinedMenuItem.new({ item: 'Paste', text: m['menu.edit.paste']() }),
			await PredefinedMenuItem.new({ item: 'SelectAll', text: m['menu.edit.select_all']() })
		]
	});

	const windowMenu = await Submenu.new({
		text: m['menu.window.title'](),
		items: [
			await PredefinedMenuItem.new({ item: 'Minimize', text: m['menu.window.minimize']() }),
			await PredefinedMenuItem.new({ item: 'Maximize', text: m['menu.window.zoom']() }),
			await PredefinedMenuItem.new({ item: 'CloseWindow', text: m['menu.window.close']() }),
			await separator(),
			await MenuItem.new({
				id: 'window_main',
				text: m['menu.window.main'](),
				action: () => void openAppWindow('main')
			}),
			await MenuItem.new({
				id: 'window_log',
				text: m['menu.window.log'](),
				action: () => void openAppWindow('log')
			}),
			await separator(),
			await PredefinedMenuItem.new({ item: 'BringAllToFront', text: m['menu.window.bring_all_to_front']() })
		]
	});

	const menu = await Menu.new({ items: [appMenu, editMenu, windowMenu] });
	const previous = await menu.setAsAppMenu();
	await windowMenu.setAsWindowsMenuForNSApp();
	await previous?.close();
}
