
export async function invoke_backup_import(){
    return Number(await window.__TAURI__.invoke("load_backup"));
}