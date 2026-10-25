#!/usr/bin/env python3
"""Local mock peer only. Does not attest actual Fallout 4 compatibility."""
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import threading

ROOT=Path(__file__).resolve().parents[1]
BIN=ROOT/'target'/'debug'/('pipboy.exe' if os.name=='nt' else 'pipboy')
def frame(kind,data=b''):
    return struct.pack('<IB',len(data),kind)+data

def exercise(mode):
    errors=[]
    with socket.socket() as server, tempfile.TemporaryDirectory() as temp:
        server.bind(('127.0.0.1',0));server.listen();server.settimeout(10)
        address='127.0.0.1:'+str(server.getsockname()[1])
        def peer():
            try:
                with server.accept()[0] as client:
                    client.settimeout(5)
                    client.sendall(frame(1,b'{"version":"fixture","lang":"en"}')+frame(0))
                    reply=b''
                    while len(reply)<5: reply+=client.recv(5-len(reply))
                    assert reply==b'\0'*5
                    if mode=='busy': client.sendall(frame(2));return
                    if mode=='oversize':client.sendall(struct.pack('<IB',9000000,3));return
                    # Root arrives before its referenced scalar, in another frame.
                    root=bytes([8,0,0,0,0,1,0,1,0,0,0])+b'HP\0'+bytes([0,0])
                    client.sendall(frame(3,root))
                    scalar=bytes([4,1,0,0,0,100,0,0,0])
                    packet=frame(3,scalar)
                    for b in packet: client.sendall(bytes([b]))
                    try:
                        assert client.recv(1)==b'' # read-only client sends no game commands
                    except ConnectionResetError:
                        pass # TCP close may reset after the final snapshot/terminal exit
            except Exception as error: errors.append(error)
        thread=threading.Thread(target=peer);thread.start()
        command=[str(BIN),'game','--connect',address,'--confirm']
        if mode=='interactive':
            if os.name=='nt':
                from smoke_windows_conpty import ConPty, configure_windows_api
                host=ConPty(configure_windows_api(),Path(temp)/'not-created',argv=command)
                try:
                    host.start();host.wait_for(b'100')
                    host.send(b'\x1b[B\r\x08\x03');host.finish(expected_code=0)
                finally:host.close()
            else:
                import fcntl,pty,termios,time,select
                master,slave=pty.openpty()
                fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,100,0,0))
                original=termios.tcgetattr(slave)
                process=subprocess.Popen(command,stdin=slave,stdout=slave,stderr=slave,env=dict(os.environ,PIPBOY_HOME=temp,TERM='xterm-256color'))
                output=b'';sent=False;deadline=time.monotonic()+10
                try:
                    while process.poll() is None and time.monotonic()<deadline:
                        if select.select([master],[],[],.05)[0]:
                            chunk=os.read(master,65536);output+=chunk
                            if b'\x1b[6n' in chunk:os.write(master,b'\x1b[1;1R')
                        if b'100' in output and not sent:
                            os.write(master,b'\x1b[B\r\x7f\x03');sent=True
                    process.wait(timeout=2)
                    while select.select([master],[],[],0)[0]:output+=os.read(master,65536)
                    assert sent and process.returncode==0,output
                    assert b'\x1b[?1049l' in output
                    assert termios.tcgetattr(slave)==original
                finally:
                    if process.poll() is None:process.kill()
                    process.wait();os.close(master);os.close(slave)
        else:
            result=subprocess.run([*command,'--snapshot'],capture_output=True,text=True,timeout=15,env=dict(os.environ,PIPBOY_HOME=temp))
        thread.join(10);assert not thread.is_alive();assert not errors,(mode,errors)
        assert not list(Path(temp).iterdir())
        if mode=='interactive':pass
        elif mode=='normal':
            assert result.returncode==0,result.stderr
            report=json.loads(result.stdout);assert report['data']=={'HP':100};assert report['source']==address
        else: assert result.returncode!=0 and not result.stdout,(mode,result)
for mode in ['normal','interactive','busy','oversize']:exercise(mode)
for args in [[],['--connect','localhost:27000','--confirm'],['--connect','127.0.0.1:27000'],['--connect','127.0.0.1:27000','--confirm','--confirm']]:
    result=subprocess.run([str(BIN),'game',*args],capture_output=True,timeout=5)
    assert result.returncode!=0 and not result.stdout
with tempfile.TemporaryDirectory() as temporary:
    env=dict(os.environ,PIPBOY_HOME=temporary)
    subprocess.run([str(BIN),'permissions','deny','network.fetch'],env=env,capture_output=True,check=True)
    policy=Path(temporary)/'permissions.json'
    original=policy.read_bytes()
    result=subprocess.run([str(BIN),'game','--connect','127.0.0.1:1','--confirm','--snapshot'],env=env,capture_output=True,text=True,timeout=5)
    assert result.returncode!=0 and 'denied by saved' in result.stderr
    assert policy.read_bytes()==original
    policy.write_text('{invalid')
    result=subprocess.run([str(BIN),'game','--connect','127.0.0.1:1','--confirm','--snapshot'],env=env,capture_output=True,text=True,timeout=5)
    assert result.returncode!=0 and 'cannot reach' not in result.stderr
    assert policy.read_text()=='{invalid'
print('PASS: fragmented game stream, heartbeat echo, late references, busy/oversize refusal, strict consent/arguments, no commands or files')
