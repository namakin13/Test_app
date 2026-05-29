import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Gamepad2 } from "lucide-react";

interface SteamGame {
  name: string;
  appid: string;
  header_image_url: string;
}

interface GameBannerProps {
  game: SteamGame;
  customIcon?: string;
  /** Steam以外の手動追加ゲームの場合 true。CDN/ローカルキャッシュ取得を行わない */
  manual?: boolean;
}

export const GameBanner = ({ game, customIcon, manual = false }: GameBannerProps) => {
  const [localCacheImg, setLocalCacheImg] = useState<string | null>(null);

  useEffect(() => {
    // 手動ゲームには Steam のローカルキャッシュ画像は存在しないため取得しない
    if (manual) {
      setLocalCacheImg(null);
      return;
    }
    invoke<string | null>("get_local_steam_image", { appid: game.appid })
      .then(res => {
        if (res) setLocalCacheImg(res);
      })
      .catch(console.error);
  }, [game.appid, manual]);

  // 正方形アイコン向けフォールバックチェーン:
  //   1. カスタムアイコン（ユーザー設定）
  //   2. ローカルキャッシュ（library_600x900.jpg → header.jpg → {hash}.jpg の順）
  //   3. CDN library_600x900.jpg（縦長＝正方形トリミングに最適）
  //   4. CDN header.jpg（DLC・ツール等で portrait がない場合の保険）
  // 手動ゲームは Steam CDN を持たないため、カスタムアイコンのみ（なければプレースホルダ）
  const cdnPortrait = `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${game.appid}/library_600x900.jpg`;
  const cdnHeader = `https://cdn.akamai.steamstatic.com/steam/apps/${game.appid}/header.jpg`;

  const fallbacks = manual
    ? (customIcon ? [customIcon] : [])
    : customIcon
    ? [customIcon, ...(localCacheImg ? [localCacheImg] : []), cdnPortrait, cdnHeader]
    : [...(localCacheImg ? [localCacheImg] : []), cdnPortrait, cdnHeader];

  const [imgIndex, setImgIndex] = useState(0);
  const [isError, setIsError] = useState(fallbacks.length === 0);

  useEffect(() => {
    setImgIndex(0);
    setIsError(fallbacks.length === 0);
  }, [game.appid, customIcon, localCacheImg, manual]);

  return (
    <div className="game-banner-container">
      {fallbacks.length > 0 && (
        <img
          src={fallbacks[imgIndex]}
          alt={game.name}
          className="game-banner"
          style={isError ? { display: 'none' } : undefined}
          onError={() => {
            if (imgIndex < fallbacks.length - 1) {
              setImgIndex(prev => prev + 1);
            } else {
              setIsError(true);
            }
          }}
        />
      )}
      <div className={`game-banner-fallback ${isError ? '' : 'hidden'}`}>
        <Gamepad2 size={48} opacity={0.2} />
      </div>
    </div>
  );
};
