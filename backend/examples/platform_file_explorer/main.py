#!/usr/bin/env python3
"""
Platform-Aware File Explorer
Designed to run in sandbox and communicate with the platform
"""
import sys
import os
import json
import threading
import gi

gi.require_version('Gtk', '3.0')
gi.require_version('GdkPixbuf', '2.0')
from gi.repository import Gtk, GdkPixbuf, Gdk, GLib

class PlatformFileExplorer(Gtk.Window):
    def __init__(self):
        super().__init__(title="File Explorer")
        self.set_default_size(1920, 1080)
        self.current_path = os.path.expanduser("~")
        
        # Setup IPC reader thread
        self.setup_ipc()
        
        # Build UI
        self.build_ui()
        
        # Load initial directory
        self.load_directory(self.current_path)
        
        # Send initial state to platform
        self.send_state()
    
    def setup_ipc(self):
        """Setup stdin reader for platform commands"""
        def read_stdin():
            for line in sys.stdin:
                try:
                    command = json.loads(line.strip())
                    GLib.idle_add(self.handle_command, command)
                except json.JSONDecodeError:
                    pass
        
        thread = threading.Thread(target=read_stdin, daemon=True)
        thread.start()
    
    def send_message(self, msg):
        """Send message to platform via stdout"""
        print(json.dumps(msg), flush=True)
    
    def send_state(self):
        """Send current state to platform"""
        selected = self.get_selected_file()
        self.send_message({
            "type": "state",
            "path": self.current_path,
            "selected": selected,
            "actions": self.get_available_actions(selected)
        })
    
    def get_available_actions(self, selected):
        """Return available actions based on context"""
        actions = ["upload"]
        if selected and os.path.isfile(selected):
            actions.append("download")
            actions.append("delete")
        elif selected and os.path.isdir(selected):
            actions.append("delete")
        return actions
    
    def handle_command(self, command):
        """Handle commands from platform"""
        cmd_type = command.get("type")
        
        if cmd_type == "upload":
            self.handle_upload(command)
        elif cmd_type == "download_request":
            self.handle_download_request()
        elif cmd_type == "delete":
            self.handle_delete()
    
    def handle_upload(self, command):
        """Handle file upload from platform"""
        filename = command.get("filename")
        data = command.get("data")  # base64 encoded
        
        if filename and data:
            import base64
            filepath = os.path.join(self.current_path, filename)
            try:
                with open(filepath, 'wb') as f:
                    f.write(base64.b64decode(data))
                self.load_directory(self.current_path)
                self.send_message({"type": "upload_complete", "filename": filename})
            except Exception as e:
                self.send_message({"type": "error", "message": str(e)})
    
    def handle_download_request(self):
        """Handle download request from platform"""
        selected = self.get_selected_file()
        if selected and os.path.isfile(selected):
            try:
                import base64
                with open(selected, 'rb') as f:
                    data = base64.b64encode(f.read()).decode('utf-8')
                self.send_message({
                    "type": "download_data",
                    "filename": os.path.basename(selected),
                    "data": data
                })
            except Exception as e:
                self.send_message({"type": "error", "message": str(e)})
    
    def handle_delete(self):
        """Handle delete request"""
        selected = self.get_selected_file()
        if selected:
            try:
                if os.path.isfile(selected):
                    os.remove(selected)
                elif os.path.isdir(selected):
                    os.rmdir(selected)
                self.load_directory(self.current_path)
                self.send_message({"type": "delete_complete"})
            except Exception as e:
                self.send_message({"type": "error", "message": str(e)})
    
    def build_ui(self):
        """Build the UI"""
        # Main container
        paned = Gtk.Paned(orientation=Gtk.Orientation.HORIZONTAL)
        self.add(paned)
        
        # Left: File list
        left_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        paned.add1(left_box)
        
        # Path bar
        path_bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.path_entry = Gtk.Entry()
        self.path_entry.set_text(self.current_path)
        self.path_entry.connect("activate", self.on_path_changed)
        path_bar.pack_start(self.path_entry, True, True, 5)
        
        up_button = Gtk.Button(label="Up")
        up_button.connect("clicked", self.on_up_clicked)
        path_bar.pack_start(up_button, False, False, 5)
        
        left_box.pack_start(path_bar, False, False, 5)
        
        # File list
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
        
        self.file_store = Gtk.ListStore(GdkPixbuf.Pixbuf, str, str, str)
        self.file_view = Gtk.TreeView(model=self.file_store)
        self.file_view.connect("row-activated", self.on_file_activated)
        self.file_view.connect("cursor-changed", self.on_selection_changed)
        
        # Icon column
        icon_renderer = Gtk.CellRendererPixbuf()
        icon_column = Gtk.TreeViewColumn("", icon_renderer, pixbuf=0)
        self.file_view.append_column(icon_column)
        
        # Name column
        name_renderer = Gtk.CellRendererText()
        name_column = Gtk.TreeViewColumn("Name", name_renderer, text=1)
        name_column.set_sort_column_id(1)
        self.file_view.append_column(name_column)
        
        # Size column
        size_renderer = Gtk.CellRendererText()
        size_column = Gtk.TreeViewColumn("Size", size_renderer, text=2)
        self.file_view.append_column(size_column)
        
        scrolled.add(self.file_view)
        left_box.pack_start(scrolled, True, True, 0)
        
        # Right: Preview pane
        self.preview_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.preview_label = Gtk.Label(label="Select a file to preview")
        self.preview_box.pack_start(self.preview_label, True, True, 0)
        paned.add2(self.preview_box)
        
        paned.set_position(600)
    
    def load_directory(self, path):
        """Load directory contents"""
        if not os.path.isdir(path):
            return
        
        self.current_path = path
        self.path_entry.set_text(path)
        self.file_store.clear()
        
        try:
            entries = os.listdir(path)
            entries.sort()
            
            # Get icons
            icon_theme = Gtk.IconTheme.get_default()
            folder_icon = icon_theme.load_icon("folder", 24, 0)
            file_icon = icon_theme.load_icon("text-x-generic", 24, 0)
            
            for entry in entries:
                if entry.startswith('.'):
                    continue
                
                full_path = os.path.join(path, entry)
                
                try:
                    stat = os.stat(full_path)
                    is_dir = os.path.isdir(full_path)
                    
                    icon = folder_icon if is_dir else file_icon
                    size = "" if is_dir else self.format_size(stat.st_size)
                    
                    self.file_store.append([icon, entry, size, full_path])
                except:
                    pass
        
        except Exception as e:
            print(f"Error loading directory: {e}", file=sys.stderr)
    
    def format_size(self, size):
        """Format file size"""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size < 1024:
                return f"{size:.1f} {unit}"
            size /= 1024
        return f"{size:.1f} TB"
    
    def get_selected_file(self):
        """Get currently selected file path"""
        selection = self.file_view.get_selection()
        model, treeiter = selection.get_selected()
        if treeiter:
            return model[treeiter][3]
        return None
    
    def on_path_changed(self, entry):
        """Handle path entry change"""
        new_path = entry.get_text()
        if os.path.isdir(new_path):
            self.load_directory(new_path)
            self.send_state()
    
    def on_up_clicked(self, button):
        """Navigate to parent directory"""
        parent = os.path.dirname(self.current_path)
        if parent != self.current_path:
            self.load_directory(parent)
            self.send_state()
    
    def on_file_activated(self, tree_view, path, column):
        """Handle file double-click"""
        model = tree_view.get_model()
        treeiter = model.get_iter(path)
        file_path = model[treeiter][3]
        
        if os.path.isdir(file_path):
            self.load_directory(file_path)
            self.send_state()
        else:
            self.preview_file(file_path)
    
    def on_selection_changed(self, tree_view):
        """Handle selection change"""
        selected = self.get_selected_file()
        if selected and os.path.isfile(selected):
            self.preview_file(selected)
        self.send_state()
    
    def preview_file(self, file_path):
        """Preview a file"""
        # Clear previous preview
        for child in self.preview_box.get_children():
            self.preview_box.remove(child)
        
        ext = os.path.splitext(file_path)[1].lower()
        
        try:
            if ext in ['.jpg', '.jpeg', '.png', '.gif', '.bmp']:
                self.preview_image(file_path)
            elif ext in ['.txt', '.md', '.py', '.rs', '.js', '.json', '.xml', '.html']:
                self.preview_text(file_path)
            else:
                label = Gtk.Label(label=f"Preview not available for {ext} files")
                self.preview_box.pack_start(label, True, True, 0)
        except Exception as e:
            label = Gtk.Label(label=f"Error previewing file: {str(e)}")
            self.preview_box.pack_start(label, True, True, 0)
        
        self.preview_box.show_all()
    
    def preview_image(self, file_path):
        """Preview an image file"""
        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_file(file_path)
            
            # Scale to fit
            max_width = 800
            max_height = 800
            width = pixbuf.get_width()
            height = pixbuf.get_height()
            
            if width > max_width or height > max_height:
                scale = min(max_width / width, max_height / height)
                new_width = int(width * scale)
                new_height = int(height * scale)
                pixbuf = pixbuf.scale_simple(new_width, new_height, GdkPixbuf.InterpType.BILINEAR)
            
            image = Gtk.Image.new_from_pixbuf(pixbuf)
            self.preview_box.pack_start(image, True, False, 10)
        except Exception as e:
            label = Gtk.Label(label=f"Error loading image: {str(e)}")
            self.preview_box.pack_start(label, True, True, 0)
    
    def preview_text(self, file_path):
        """Preview a text file"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read(10000)  # Limit to 10KB
            
            scrolled = Gtk.ScrolledWindow()
            scrolled.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.AUTOMATIC)
            
            text_view = Gtk.TextView()
            text_view.set_editable(False)
            text_view.get_buffer().set_text(content)
            text_view.set_monospace(True)
            
            scrolled.add(text_view)
            self.preview_box.pack_start(scrolled, True, True, 0)
        except Exception as e:
            label = Gtk.Label(label=f"Error loading text: {str(e)}")
            self.preview_box.pack_start(label, True, True, 0)

def main():
    app = PlatformFileExplorer()
    app.connect("destroy", Gtk.main_quit)
    app.show_all()
    Gtk.main()

if __name__ == "__main__":
    main()
