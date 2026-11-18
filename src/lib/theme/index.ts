// 読み込む順序に意味がある：Carbonの土台 → ブランドの上書き（tokens.css）→ 書体 → アプリ全体の基本
import './carbon_theme.scss';
import './tokens.css';
import './fonts.css';
import './base.css';

export { applyTheme, type ThemePreference } from './theme';
