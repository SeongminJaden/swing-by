/**
 * AI가 생성한 코드를 파일 시스템에 기록하는 서비스
 */

import { ParsedCodeBlock } from '../utils/codeParser';
import { useFileStore } from '../stores/fileStore';

export interface WriteResult {
  filePath: string;
  fullPath: string;
  success: boolean;
  error?: string;
  isNew: boolean;
}

/**
 * 파싱된 코드 블록들을 워크스페이스에 파일로 저장합니다.
 * 디렉토리가 없으면 자동 생성합니다.
 */
export async function writeCodeBlocks(
  codeBlocks: ParsedCodeBlock[],
  workspacePath: string
): Promise<WriteResult[]> {
  const api = window.electronAPI;
  if (!api) {
    return codeBlocks.map((block) => ({
      filePath: block.filePath,
      fullPath: `${workspacePath}/${block.filePath}`,
      success: false,
      error: 'electronAPI를 사용할 수 없습니다',
      isNew: true,
    }));
  }

  const results: WriteResult[] = [];

  for (const block of codeBlocks) {
    const fullPath = normalizePath(`${workspacePath}/${block.filePath}`);

    try {
      // 파일이 이미 존재하는지 확인
      const exists = await api.fileExists(fullPath);

      // 디렉토리 생성 (필요 시)
      const dirPath = fullPath.substring(0, fullPath.lastIndexOf('/'));
      if (dirPath) {
        await api.mkdir(dirPath);
      }

      // 파일 쓰기
      const success = await api.writeFile(fullPath, block.code);

      results.push({
        filePath: block.filePath,
        fullPath,
        success,
        isNew: !exists,
      });
    } catch (err: any) {
      results.push({
        filePath: block.filePath,
        fullPath,
        success: false,
        error: err.message || '파일 쓰기 실패',
        isNew: true,
      });
    }
  }

  // 파일 트리 새로고침
  try {
    await useFileStore.getState().refreshTree();
  } catch {
    // 새로고침 실패는 무시
  }

  return results;
}

/**
 * 단일 코드 블록을 파일로 저장합니다.
 */
export async function writeSingleCodeBlock(
  block: ParsedCodeBlock,
  workspacePath: string
): Promise<WriteResult> {
  const results = await writeCodeBlocks([block], workspacePath);
  return results[0];
}

/**
 * 경로 정규화 (중복 슬래시 제거)
 */
function normalizePath(p: string): string {
  return p.replace(/\/+/g, '/');
}

/**
 * 파일을 에디터에서 엽니다.
 */
export async function openFileInEditor(fullPath: string): Promise<void> {
  const fileStore = useFileStore.getState();
  await fileStore.openFile(fullPath);
}
