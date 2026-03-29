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
}

export const GameBanner = ({ game, customIcon }: GameBannerProps) => {
  const [localCacheImg, setLocalCacheImg] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("get_local_steam_image", { appid: game.appid })
      .then(res => {
        if (res) setLocalCacheImg(res);
      })
      .catch(console.error);
  }, [game.appid]);

  // 正方形アイコン向けフォールバックチェーン:
  //   1. カスタムアイコン（ユーザー設定）
  //   2. ローカルキャッシュ（library_600x900.jpg → header.jpg → {hash}.jpg の順）
  //   3. CDN library_600x900.jpg（縦長＝正方形トリミングに最適）
  //   4. CDN header.jpg（DLC・ツール等で portrait がない場合の保険）
  const cdnPortrait = `https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/${game.appid}/library_600x900.jpg`;
  const cdnHeader = `https://cdn.akamai.steamstatic.com/steam/apps/${game.appid}/header.jpg`;

  const fallbacks = customIcon ? [
    customIcon,
    ...(localCacheImg ? [localCacheImg] : []),
    cdnPortrait,
    cdnHeader,
  ] : [
    ...(localCacheImg ? [localCacheImg] : []),
    cdnPortrait,
    cdnHeader,
  ];

  const [imgIndex, setImgIndex] = useState(0);
  const [isError, setIsError] = useState(false);

  useEffect(() => {
    setImgIndex(0);
    setIsError(false);
  }, [game.appid, customIcon, localCacheImg]);

  return (
    <div className="game-banner-container">
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
      <div className={`game-banner-fallback ${isError ? '' : 'hidden'}`}>
        <Gamepad2 size={48} opacity={0.2} />
      </div>
    </div>
  );
};
