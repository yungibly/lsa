#!/usr/bin/env python3
"""Grid artwork and one-row long previews, using actual Kitty payloads and text."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import re
from check_pty import APC, CSI, ROOT, BIN, capture, images, check_cursor
from check_layout import SGR, OSC


def run(args, **kwargs):
    code,data,err,_ = capture(args,**kwargs)
    assert code == 0 and not err, (code,err)
    return data


def replay(data):
    """Logical rows of ASCII text, including reserved rows and cursor movement."""
    data = SGR.sub(b"",OSC.sub(b"",data))
    lines = {}
    row = column = offset = 0
    while offset < len(data):
        apc = APC.match(data,offset)
        csi = CSI.match(data,offset)
        if apc: offset = apc.end(); continue
        if csi:
            count, op = int(csi[0][2:-1]), csi[0][-1:]
            if op == b"A": row -= count
            elif op == b"B": row += count
            else: column = count-1
            assert row >= 0
            offset=csi.end();continue
        value=data[offset]
        if value == 13: column=0
        elif value == 10: row += 1
        else:
            assert 32 <= value < 127, "ASCII replay fixtures required"
            line=lines.setdefault(row,{})
            line[column]=chr(value)
            column += 1
        offset += 1
    return [''.join(lines.get(y,{}).get(x,' ') for x in range(max(lines.get(y,{0:''}))+1)).rstrip() for y in range(row)]


def main():
    cases=0
    with tempfile.TemporaryDirectory(dir=ROOT/'target',prefix='thumbnail-ux-') as temp:
        root=Path(temp)
        mixed=root/'mixed';mixed.mkdir()
        for name in ['folder','other-folder']: (mixed/name).mkdir()
        (mixed/'broken.png').write_text('not an image')
        (mixed/'unknown.weird').touch()
        (mixed/'unsupported.HEIC').touch()
        shutil.copyfile(ROOT/'img-test/generated/still.gif',mixed/'photo.GIF')
        (mixed/'link.png').symlink_to('missing.png')
        data=run(['--grid',mixed],decorated=True,cols=122,rows=40,pixels=(976,680))
        frames=images(data,pixels=True)
        assert len(frames)==7
        assert all(int(c[b'r'])==3 and int(c[b'c'])==14 for c,_ in frames)
        assert all(int(c[b's'])==112 and int(c[b'v'])==51 for c,_ in frames)
        # Every represented entry has visible artwork/pixels. Distinct semantic
        # fallbacks stay distinct; broken and dangling image sources share error art.
        for control,pixels in frames:
            assert sum(alpha>64 for alpha in pixels[3::4]) > len(pixels)//40
        payloads=[pixels for _,pixels in frames]
        assert payloads[0]==payloads[2]  # broken.png, link.png
        assert payloads[1]==payloads[3]  # folder, other-folder
        assert payloads[0]!=payloads[1]!=payloads[5]
        assert payloads[5]!=payloads[6]  # unknown versus unsupported image type
        assert b'[dir]' not in data and b'[no preview]' not in data
        text=SGR.sub(b'',APC.sub(b'',CSI.sub(b'',data))).decode()
        assert all(not 0xe000<=ord(c)<=0xf8ff for c in text), 'grid label has duplicate font icon'
        for name in [p.name for p in mixed.iterdir()]: assert len(re.findall(r'(?<!\S)'+re.escape(name)+r'(?!\S)',text))==1
        cases+=1

        # Case-insensitive GIF classification, even on an accidentally executable
        # image, and a usable generic icon/color for unknown extensions.
        (mixed/'photo.GIF').chmod(0o755)
        data=run(['-1',mixed],decorated=True,cols=122)
        assert '\uf1c5 photo.GIF'.encode() in data and b'\x1b[35m' in data
        assert '\uf15b unknown.weird'.encode() in data and b'\x1b[37m' in data
        cases+=1

        entries=root/'long';entries.mkdir()
        (entries/'a-folder').mkdir()
        (entries/'b-broken.png').write_text('bad')
        shutil.copyfile(ROOT/'img-test/generated/landscape.png',entries/'c-photo.png')
        shutil.copyfile(ROOT/'img-test/generated/still.gif',entries/'d-photo.gif')
        (entries/'e-link.png').symlink_to('c-photo.png')
        (entries/'f-notes.txt').touch()
        geometry=dict(cols=122,rows=8,pixels=(976,136))
        for fields in ['--fields=size','-l','--header']:
            for tty_input in [False,True]:
                data=run([fields,entries],tty_input=tty_input,**geometry)
                frames=images(data)
                assert len(frames)==4
                assert all(int(c[b'r'])==1 and int(c[b'c'])==3 for c in frames)
                assert all(int(c[b's'])==24 and int(c[b'v'])==17 for c in frames)
                assert check_cursor(data,122,8,7)==4
                lines=replay(data)
                assert len(lines)==(7 if fields=='--header' else 6), lines
                body=lines[1:] if fields=='--header' else lines
                names=sorted(p.name for p in entries.iterdir())
                offsets=[line.index(name) for name,line in zip(names,body)]
                assert len(set(offsets))==1, body
                assert all(line.count(name)==1 for name,line in zip(names,body))
                assert body[-2].endswith('e-link.png@ -> c-photo.png')
                cases+=1
        # After a limit, the name gutter stays aligned and later sources are not
        # decoded. No extra physical row is added for a miniature.
        data=run(['--fields=size','--preview-limit=2',entries],**geometry)
        assert len(images(data))==2 and len(replay(data))==6
        cases+=1
        data=run(['--fields=size',entries,entries],**geometry)
        assert len(images(data))==8
        data=run(['--fields=size','--preview-limit=3',entries,entries],**geometry)
        assert len(images(data))==3
        cases+=1
        for flags,extra in [(['--no-images'],{}),(['-1'],{}),(['--protocol=none'],{}),(['--preview-limit=0'],{}),([],{"cols":16}),([],{"rows":2}),([],{"environment":{"TMUX":"test"}}),([],{"environment":{"TERM":"dumb"}})]:
            options={**geometry,**extra}
            data=run(['--fields=size',*flags,entries],**options)
            assert not images(data) and b'\x1b' not in data
            cases+=1
        piped=subprocess.run([str(BIN),'-l',str(entries)],capture_output=True,check=True)
        assert b'\x1b' not in piped.stdout and len(piped.stdout.splitlines())==6
        cases+=1
        # Warm cache keeps exact tiny pixels and text, and uses its own geometry
        # key rather than accidentally reusing a full grid thumbnail.
        cache=root/'cache';flag=f'--cache-dir={cache}'
        run([flag,'--grid',entries],**geometry)
        def cached():
            code,data,err,_=capture([flag,'--cache-stats','--fields=size',entries],**geometry)
            assert code==0
            return data,err
        cold,cold_stats=cached();warm,warm_stats=cached()
        assert cold==warm==run(['--fields=size',entries],**geometry)
        assert b'1 hits, 3 misses, 2 writes, 0 errors' in cold_stats, cold_stats
        assert b'3 hits, 1 misses, 0 writes, 0 errors' in warm_stats, warm_stats
        cases+=1
        # A directory made entirely of ordinary files still has artwork with
        # explicit --grid, but icon traffic cannot grow past the placement cap.
        many=root/'many';many.mkdir()
        for i in range(300): (many/f'file-{i:03}').touch()
        data=run(['--grid','--preview-limit=256',many],cols=80,rows=8,pixels=(80,8))
        assert 250<=len(images(data))<=256
        assert sum(len(m[0]) for m in APC.finditer(data))<=8*1024*1024
        plain=APC.sub(b'',data)
        for i in range(300): assert plain.count(f'file-{i:03}'.encode())==1
        cases+=1
    print(f'{cases} thumbnail UX checks passed: visible artwork, GIF styling, compact grid, one-row long previews, alignment, budgets, and cache geometry.')


if __name__=='__main__': main()
