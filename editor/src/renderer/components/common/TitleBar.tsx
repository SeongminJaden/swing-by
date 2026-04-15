import { Minus, Square, X } from 'lucide-react';

export function TitleBar() {
  const isMac = navigator.platform.toLowerCase().includes('mac');

  return (
    <div className="ide-titlebar">
      {/* macOS: traffic lights space */}
      {isMac && <div className="ide-titlebar-traffic-lights" />}

      {/* Center: title */}
      <div className="ide-titlebar-title">
        <span>VidEplace — 쇼핑몰</span>
      </div>

      {/* Windows/Linux: window controls */}
      {!isMac && (
        <div className="flex titlebar-no-drag">
          <button className="ide-titlebar-control">
            <Minus size={14} />
          </button>
          <button className="ide-titlebar-control">
            <Square size={12} />
          </button>
          <button className="ide-titlebar-control ide-titlebar-control-close">
            <X size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
