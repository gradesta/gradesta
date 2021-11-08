# addressbook_fast.pyx
# distutils: language = c++
# distutils: include_dirs = /home/timothy/.cache/pypoetry/virtualenvs/gradesta-level0-fvVCVjj9-py3.7/lib/python3.7/site-packages
# distutils: libraries = capnpc capnp capnp-rpc
# distutils: sources = level0.capnp.cpp
# cython: c_string_type = str
# cython: c_string_encoding = default
# cython: embedsignature = True


# TODO: add struct/enum/list types







import capnp
import level0_capnp

from capnp.includes.types cimport *
from capnp cimport helpers
from capnp.includes.capnp_cpp cimport DynamicValue, Schema, VOID, StringPtr, ArrayPtr, Data
from capnp.lib.capnp cimport _DynamicStructReader, _DynamicStructBuilder, _DynamicListBuilder, _DynamicEnum, _StructSchemaField, to_python_builder, to_python_reader, _to_dict, _setDynamicFieldStatic, _Schema, _InterfaceSchema

from capnp.helpers.non_circular cimport reraise_kj_exception

cdef DynamicValue.Reader _extract_dynamic_struct_builder(_DynamicStructBuilder value):
    return DynamicValue.Reader(value.thisptr.asReader())

cdef DynamicValue.Reader _extract_dynamic_struct_reader(_DynamicStructReader value):
    return DynamicValue.Reader(value.thisptr)

cdef DynamicValue.Reader _extract_dynamic_enum(_DynamicEnum value):
    return DynamicValue.Reader(value.thisptr)

cdef _from_list(_DynamicListBuilder msg, list d):
    cdef size_t count = 0
    for val in d:
        msg._set(count, val)
        count += 1


cdef extern from "level0.capnp.h":
    Schema getAddressSchema"capnp::Schema::from<Address>"()

    cdef cppclass Address"Address":
        cppclass Reader:
            StringPtr getSocket() except +reraise_kj_exception
                
            StringPtr getLocale() except +reraise_kj_exception
                
            StringPtr getServiceName() except +reraise_kj_exception
                
            DynamicValue.Reader getVertexPath() except +reraise_kj_exception
            DynamicValue.Reader getQargs() except +reraise_kj_exception
            DynamicValue.Reader getQvals() except +reraise_kj_exception
            uint64_t getIdentity() except +reraise_kj_exception
                
        cppclass Builder:
            StringPtr getSocket() except +reraise_kj_exception
                
            void setSocket(StringPtr) except +reraise_kj_exception
                
            StringPtr getLocale() except +reraise_kj_exception
                
            void setLocale(StringPtr) except +reraise_kj_exception
                
            StringPtr getServiceName() except +reraise_kj_exception
                
            void setServiceName(StringPtr) except +reraise_kj_exception
                
            DynamicValue.Builder getVertexPath() except +reraise_kj_exception
            void setVertexPath(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getQargs() except +reraise_kj_exception
            void setQargs(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getQvals() except +reraise_kj_exception
            void setQvals(DynamicValue.Reader) except +reraise_kj_exception
            uint64_t getIdentity() except +reraise_kj_exception
                
            void setIdentity(uint64_t) except +reraise_kj_exception
                
    Schema getTimeSchema"capnp::Schema::from<Time>"()

    cdef cppclass Time"Time":
        cppclass Reader:
            int64_t getTimeTaiSecs() except +reraise_kj_exception
                
            int64_t getTimeTaiNs() except +reraise_kj_exception
                
        cppclass Builder:
            int64_t getTimeTaiSecs() except +reraise_kj_exception
                
            void setTimeTaiSecs(int64_t) except +reraise_kj_exception
                
            int64_t getTimeTaiNs() except +reraise_kj_exception
                
            void setTimeTaiNs(int32_t) except +reraise_kj_exception
                
    Schema getVertexSchema"capnp::Schema::from<Vertex>"()

    cdef cppclass Vertex"Vertex":
        cppclass Reader:
            DynamicValue.Reader getAddress() except +reraise_kj_exception
            uint64_t getInstanceId() except +reraise_kj_exception
                
            StringPtr getView() except +reraise_kj_exception
                
        cppclass Builder:
            DynamicValue.Builder getAddress() except +reraise_kj_exception
            void setAddress(DynamicValue.Reader) except +reraise_kj_exception
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            StringPtr getView() except +reraise_kj_exception
                
            void setView(StringPtr) except +reraise_kj_exception
                
    Schema getVertexMessageSchema"capnp::Schema::from<VertexMessage>"()

    cdef cppclass VertexMessage"VertexMessage":
        cppclass Reader:
            uint64_t getInstanceId() except +reraise_kj_exception
                
            Data.Reader getData() except +reraise_kj_exception
                
        cppclass Builder:
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            Data.Builder getData() except +reraise_kj_exception
                
            void setData(ArrayPtr[byte]) except +reraise_kj_exception
                
    Schema getDataUpdateSchema"capnp::Schema::from<DataUpdate>"()

    cdef cppclass DataUpdate"DataUpdate":
        cppclass Reader:
            int64_t getUpdateId() except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            StringPtr getMime() except +reraise_kj_exception
                
            Data.Reader getData() except +reraise_kj_exception
                
        cppclass Builder:
            int64_t getUpdateId() except +reraise_kj_exception
                
            void setUpdateId(int64_t) except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            StringPtr getMime() except +reraise_kj_exception
                
            void setMime(StringPtr) except +reraise_kj_exception
                
            Data.Builder getData() except +reraise_kj_exception
                
            void setData(ArrayPtr[byte]) except +reraise_kj_exception
                
    Schema getEncryptionUpdateSchema"capnp::Schema::from<EncryptionUpdate>"()

    cdef cppclass EncryptionUpdate"EncryptionUpdate":
        cppclass Reader:
            int64_t getUpdateId() except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            StringPtr getKeys() except +reraise_kj_exception
                
        cppclass Builder:
            int64_t getUpdateId() except +reraise_kj_exception
                
            void setUpdateId(int64_t) except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            StringPtr getKeys() except +reraise_kj_exception
                
            void setKeys(StringPtr) except +reraise_kj_exception
                
    Schema getPortUpdateSchema"capnp::Schema::from<PortUpdate>"()

    cdef cppclass PortUpdate"PortUpdate":
        cppclass Reader:
            int64_t getUpdateId() except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            int64_t getDirection() except +reraise_kj_exception
                
            DynamicValue.Reader getConnectedVertex() except +reraise_kj_exception
        cppclass Builder:
            int64_t getUpdateId() except +reraise_kj_exception
                
            void setUpdateId(int64_t) except +reraise_kj_exception
                
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            int64_t getDirection() except +reraise_kj_exception
                
            void setDirection(int64_t) except +reraise_kj_exception
                
            DynamicValue.Builder getConnectedVertex() except +reraise_kj_exception
            void setConnectedVertex(DynamicValue.Reader) except +reraise_kj_exception
    Schema getPortUpdate_connectedVertexSchema"capnp::Schema::from<PortUpdate::ConnectedVertex>"()

    cdef cppclass PortUpdate_connectedVertex"PortUpdate::ConnectedVertex":
        cppclass Reader:
            void getDisconnected() except +reraise_kj_exception
                
            void getClosed() except +reraise_kj_exception
                
            DynamicValue.Reader getVertex() except +reraise_kj_exception
            DynamicValue.Reader getSymlink() except +reraise_kj_exception
        cppclass Builder:
            void getDisconnected() except +reraise_kj_exception
                
            void setDisconnected(DynamicValue.Reader) except +reraise_kj_exception
            void getClosed() except +reraise_kj_exception
                
            void setClosed(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getVertex() except +reraise_kj_exception
            void setVertex(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getSymlink() except +reraise_kj_exception
            void setSymlink(DynamicValue.Reader) except +reraise_kj_exception
    Schema getVertexStateSchema"capnp::Schema::from<VertexState>"()

    cdef cppclass VertexState"VertexState":
        cppclass Reader:
            uint64_t getInstanceId() except +reraise_kj_exception
                
            uint64_t getStatus() except +reraise_kj_exception
                
            cbool getReaped() except +reraise_kj_exception
                
        cppclass Builder:
            uint64_t getInstanceId() except +reraise_kj_exception
                
            void setInstanceId(uint64_t) except +reraise_kj_exception
                
            uint64_t getStatus() except +reraise_kj_exception
                
            void setStatus(uint64_t) except +reraise_kj_exception
                
            cbool getReaped() except +reraise_kj_exception
                
            void setReaped(cbool) except +reraise_kj_exception
                
    Schema getUpdateStatusSchema"capnp::Schema::from<UpdateStatus>"()

    cdef cppclass UpdateStatus"UpdateStatus":
        cppclass Reader:
            uint64_t getUpdateId() except +reraise_kj_exception
                
            uint64_t getStatus() except +reraise_kj_exception
                
            DynamicValue.Reader getExplanation() except +reraise_kj_exception
        cppclass Builder:
            uint64_t getUpdateId() except +reraise_kj_exception
                
            void setUpdateId(uint64_t) except +reraise_kj_exception
                
            uint64_t getStatus() except +reraise_kj_exception
                
            void setStatus(uint64_t) except +reraise_kj_exception
                
            DynamicValue.Builder getExplanation() except +reraise_kj_exception
            void setExplanation(DynamicValue.Reader) except +reraise_kj_exception
    Schema getForClientSchema"capnp::Schema::from<ForClient>"()

    cdef cppclass ForClient"ForClient":
        cppclass Reader:
            DynamicValue.Reader getVertexMessages() except +reraise_kj_exception
            DynamicValue.Reader getVertexes() except +reraise_kj_exception
            DynamicValue.Reader getVertexStates() except +reraise_kj_exception
            DynamicValue.Reader getUpdateStatuses() except +reraise_kj_exception
            DynamicValue.Reader getPortUpdates() except +reraise_kj_exception
            DynamicValue.Reader getDataUpdates() except +reraise_kj_exception
            DynamicValue.Reader getEncryptionUpdates() except +reraise_kj_exception
            DynamicValue.Reader getTimestamp() except +reraise_kj_exception
        cppclass Builder:
            DynamicValue.Builder getVertexMessages() except +reraise_kj_exception
            void setVertexMessages(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getVertexes() except +reraise_kj_exception
            void setVertexes(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getVertexStates() except +reraise_kj_exception
            void setVertexStates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getUpdateStatuses() except +reraise_kj_exception
            void setUpdateStatuses(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getPortUpdates() except +reraise_kj_exception
            void setPortUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getDataUpdates() except +reraise_kj_exception
            void setDataUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getEncryptionUpdates() except +reraise_kj_exception
            void setEncryptionUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getTimestamp() except +reraise_kj_exception
            void setTimestamp(DynamicValue.Reader) except +reraise_kj_exception
    Schema getForServiceSchema"capnp::Schema::from<ForService>"()

    cdef cppclass ForService"ForService":
        cppclass Reader:
            DynamicValue.Reader getVertexMessages() except +reraise_kj_exception
            DynamicValue.Reader getPortUpdates() except +reraise_kj_exception
            DynamicValue.Reader getDataUpdates() except +reraise_kj_exception
            DynamicValue.Reader getEncryptionUpdates() except +reraise_kj_exception
            DynamicValue.Reader getSelect() except +reraise_kj_exception
            DynamicValue.Reader getDeselect() except +reraise_kj_exception
            DynamicValue.Reader getTimestamp() except +reraise_kj_exception
        cppclass Builder:
            DynamicValue.Builder getVertexMessages() except +reraise_kj_exception
            void setVertexMessages(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getPortUpdates() except +reraise_kj_exception
            void setPortUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getDataUpdates() except +reraise_kj_exception
            void setDataUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getEncryptionUpdates() except +reraise_kj_exception
            void setEncryptionUpdates(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getSelect() except +reraise_kj_exception
            void setSelect(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getDeselect() except +reraise_kj_exception
            void setDeselect(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getTimestamp() except +reraise_kj_exception
            void setTimestamp(DynamicValue.Reader) except +reraise_kj_exception
    Schema getMessageSchema"capnp::Schema::from<Message>"()

    cdef cppclass Message"Message":
        cppclass Reader:
            DynamicValue.Reader getForClient() except +reraise_kj_exception
            DynamicValue.Reader getForService() except +reraise_kj_exception
        cppclass Builder:
            DynamicValue.Builder getForClient() except +reraise_kj_exception
            void setForClient(DynamicValue.Reader) except +reraise_kj_exception
            DynamicValue.Builder getForService() except +reraise_kj_exception
            void setForService(DynamicValue.Reader) except +reraise_kj_exception

    cdef cppclass C_DynamicStruct_Reader" ::capnp::DynamicStruct::Reader":
        Address.Reader asAddress"as<Address>"()
        Time.Reader asTime"as<Time>"()
        Vertex.Reader asVertex"as<Vertex>"()
        VertexMessage.Reader asVertexMessage"as<VertexMessage>"()
        DataUpdate.Reader asDataUpdate"as<DataUpdate>"()
        EncryptionUpdate.Reader asEncryptionUpdate"as<EncryptionUpdate>"()
        PortUpdate.Reader asPortUpdate"as<PortUpdate>"()
        PortUpdate_connectedVertex.Reader asPortUpdate_connectedVertex"as<PortUpdate::ConnectedVertex>"()
        VertexState.Reader asVertexState"as<VertexState>"()
        UpdateStatus.Reader asUpdateStatus"as<UpdateStatus>"()
        ForClient.Reader asForClient"as<ForClient>"()
        ForService.Reader asForService"as<ForService>"()
        Message.Reader asMessage"as<Message>"()

    cdef cppclass C_DynamicStruct_Builder" ::capnp::DynamicStruct::Builder":
        Address.Builder asAddress"as<Address>"()
        Time.Builder asTime"as<Time>"()
        Vertex.Builder asVertex"as<Vertex>"()
        VertexMessage.Builder asVertexMessage"as<VertexMessage>"()
        DataUpdate.Builder asDataUpdate"as<DataUpdate>"()
        EncryptionUpdate.Builder asEncryptionUpdate"as<EncryptionUpdate>"()
        PortUpdate.Builder asPortUpdate"as<PortUpdate>"()
        PortUpdate_connectedVertex.Builder asPortUpdate_connectedVertex"as<PortUpdate::ConnectedVertex>"()
        VertexState.Builder asVertexState"as<VertexState>"()
        UpdateStatus.Builder asUpdateStatus"as<UpdateStatus>"()
        ForClient.Builder asForClient"as<ForClient>"()
        ForService.Builder asForService"as<ForService>"()
        Message.Builder asMessage"as<Message>"()

_Address_Schema = _Schema()._init(getAddressSchema()).as_struct()
level0_capnp.Address.schema = _Address_Schema

cdef class Address_Reader(_DynamicStructReader):
    cdef Address.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asAddress()
    

    cpdef _get_socket(self):
        temp = self.thisptr_child.getSocket()
        return (<char*>temp.begin())[:temp.size()]
        

    property socket:
        def __get__(self):
            return self._get_socket()

    cpdef _get_locale(self):
        temp = self.thisptr_child.getLocale()
        return (<char*>temp.begin())[:temp.size()]
        

    property locale:
        def __get__(self):
            return self._get_locale()

    cpdef _get_serviceName(self):
        temp = self.thisptr_child.getServiceName()
        return (<char*>temp.begin())[:temp.size()]
        

    property serviceName:
        def __get__(self):
            return self._get_serviceName()

    cpdef _get_vertexPath(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertexPath()
        return to_python_reader(temp, self._parent)
        

    property vertexPath:
        def __get__(self):
            return self._get_vertexPath()

    cpdef _get_qargs(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getQargs()
        return to_python_reader(temp, self._parent)
        

    property qargs:
        def __get__(self):
            return self._get_qargs()

    cpdef _get_qvals(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getQvals()
        return to_python_reader(temp, self._parent)
        

    property qvals:
        def __get__(self):
            return self._get_qvals()

    cpdef _get_identity(self):
        return self.thisptr_child.getIdentity()
        

    property identity:
        def __get__(self):
            return self._get_identity()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'socket': _to_dict(self.socket, verbose, ordered),
        
        
        'locale': _to_dict(self.locale, verbose, ordered),
        
        
        'serviceName': _to_dict(self.serviceName, verbose, ordered),
        
        
        'vertexPath': _to_dict(self.vertexPath, verbose, ordered),
        
        
        'qargs': _to_dict(self.qargs, verbose, ordered),
        
        
        'qvals': _to_dict(self.qvals, verbose, ordered),
        
        
        'identity': _to_dict(self.identity, verbose, ordered),
        
        }

        

        return ret

cdef class Address_Builder(_DynamicStructBuilder):
    cdef Address.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asAddress()
    
    cpdef _get_socket(self):
        temp = self.thisptr_child.getSocket()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_socket(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setSocket(temp_string)
        

    property socket:
        def __get__(self):
            return self._get_socket()
        def __set__(self, value):
            self._set_socket(value)
    cpdef _get_locale(self):
        temp = self.thisptr_child.getLocale()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_locale(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setLocale(temp_string)
        

    property locale:
        def __get__(self):
            return self._get_locale()
        def __set__(self, value):
            self._set_locale(value)
    cpdef _get_serviceName(self):
        temp = self.thisptr_child.getServiceName()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_serviceName(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setServiceName(temp_string)
        

    property serviceName:
        def __get__(self):
            return self._get_serviceName()
        def __set__(self, value):
            self._set_serviceName(value)
    cpdef _get_vertexPath(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertexPath()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertexPath(self, list value):
        cdef uint i = 0
        self.init("vertexPath", len(value))
        cdef _DynamicListBuilder temp =  self._get_vertexPath()
        for elem in value:
            temp[i] = elem
            i += 1
        

    property vertexPath:
        def __get__(self):
            return self._get_vertexPath()
        def __set__(self, value):
            self._set_vertexPath(value)
    cpdef _get_qargs(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getQargs()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_qargs(self, list value):
        cdef uint i = 0
        self.init("qargs", len(value))
        cdef _DynamicListBuilder temp =  self._get_qargs()
        for elem in value:
            temp[i] = elem
            i += 1
        

    property qargs:
        def __get__(self):
            return self._get_qargs()
        def __set__(self, value):
            self._set_qargs(value)
    cpdef _get_qvals(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getQvals()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_qvals(self, list value):
        cdef uint i = 0
        self.init("qvals", len(value))
        cdef _DynamicListBuilder temp =  self._get_qvals()
        for elem in value:
            temp[i] = elem
            i += 1
        

    property qvals:
        def __get__(self):
            return self._get_qvals()
        def __set__(self, value):
            self._set_qvals(value)
    cpdef _get_identity(self):
        return self.thisptr_child.getIdentity()
        
    cpdef _set_identity(self, uint64_t value):
        self.thisptr_child.setIdentity(value)
        

    property identity:
        def __get__(self):
            return self._get_identity()
        def __set__(self, value):
            self._set_identity(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'socket': _to_dict(self.socket, verbose, ordered),
        
        
        'locale': _to_dict(self.locale, verbose, ordered),
        
        
        'serviceName': _to_dict(self.serviceName, verbose, ordered),
        
        
        'vertexPath': _to_dict(self.vertexPath, verbose, ordered),
        
        
        'qargs': _to_dict(self.qargs, verbose, ordered),
        
        
        'qvals': _to_dict(self.qvals, verbose, ordered),
        
        
        'identity': _to_dict(self.identity, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "socket":
                try:
                    self._set_socket(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_socket(val)
                    else:
                        raise
            elif key == "locale":
                try:
                    self._set_locale(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_locale(val)
                    else:
                        raise
            elif key == "serviceName":
                try:
                    self._set_serviceName(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_serviceName(val)
                    else:
                        raise
            elif key == "vertexPath":
                try:
                    self._set_vertexPath(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertexPath(val)
                    else:
                        raise
            elif key == "qargs":
                try:
                    self._set_qargs(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_qargs(val)
                    else:
                        raise
            elif key == "qvals":
                try:
                    self._set_qvals(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_qvals(val)
                    else:
                        raise
            elif key == "identity":
                try:
                    self._set_identity(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_identity(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(14115788365236628968, (Address_Reader, Address_Builder))


_Time_Schema = _Schema()._init(getTimeSchema()).as_struct()
level0_capnp.Time.schema = _Time_Schema

cdef class Time_Reader(_DynamicStructReader):
    cdef Time.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asTime()
    

    cpdef _get_timeTaiSecs(self):
        return self.thisptr_child.getTimeTaiSecs()
        

    property timeTaiSecs:
        def __get__(self):
            return self._get_timeTaiSecs()

    cpdef _get_timeTaiNs(self):
        return self.thisptr_child.getTimeTaiNs()
        

    property timeTaiNs:
        def __get__(self):
            return self._get_timeTaiNs()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'timeTaiSecs': _to_dict(self.timeTaiSecs, verbose, ordered),
        
        
        'timeTaiNs': _to_dict(self.timeTaiNs, verbose, ordered),
        
        }

        

        return ret

cdef class Time_Builder(_DynamicStructBuilder):
    cdef Time.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asTime()
    
    cpdef _get_timeTaiSecs(self):
        return self.thisptr_child.getTimeTaiSecs()
        
    cpdef _set_timeTaiSecs(self, int64_t value):
        self.thisptr_child.setTimeTaiSecs(value)
        

    property timeTaiSecs:
        def __get__(self):
            return self._get_timeTaiSecs()
        def __set__(self, value):
            self._set_timeTaiSecs(value)
    cpdef _get_timeTaiNs(self):
        return self.thisptr_child.getTimeTaiNs()
        
    cpdef _set_timeTaiNs(self, int32_t value):
        self.thisptr_child.setTimeTaiNs(value)
        

    property timeTaiNs:
        def __get__(self):
            return self._get_timeTaiNs()
        def __set__(self, value):
            self._set_timeTaiNs(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'timeTaiSecs': _to_dict(self.timeTaiSecs, verbose, ordered),
        
        
        'timeTaiNs': _to_dict(self.timeTaiNs, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "timeTaiSecs":
                try:
                    self._set_timeTaiSecs(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_timeTaiSecs(val)
                    else:
                        raise
            elif key == "timeTaiNs":
                try:
                    self._set_timeTaiNs(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_timeTaiNs(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(10525926821988032905, (Time_Reader, Time_Builder))


_Vertex_Schema = _Schema()._init(getVertexSchema()).as_struct()
level0_capnp.Vertex.schema = _Vertex_Schema

cdef class Vertex_Reader(_DynamicStructReader):
    cdef Vertex.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asVertex()
    

    cpdef _get_address(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getAddress()
        return to_python_reader(temp, self._parent)
        

    property address:
        def __get__(self):
            return self._get_address()

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_view(self):
        temp = self.thisptr_child.getView()
        return (<char*>temp.begin())[:temp.size()]
        

    property view:
        def __get__(self):
            return self._get_view()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'address': _to_dict(self.address, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'view': _to_dict(self.view, verbose, ordered),
        
        }

        

        return ret

cdef class Vertex_Builder(_DynamicStructBuilder):
    cdef Vertex.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asVertex()
    
    cpdef _get_address(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getAddress()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_address(self, value):
        _setDynamicFieldStatic(self.thisptr, "address", value, self._parent)
        

    property address:
        def __get__(self):
            return self._get_address()
        def __set__(self, value):
            self._set_address(value)
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_view(self):
        temp = self.thisptr_child.getView()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_view(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setView(temp_string)
        

    property view:
        def __get__(self):
            return self._get_view()
        def __set__(self, value):
            self._set_view(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'address': _to_dict(self.address, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'view': _to_dict(self.view, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "address":
                try:
                    self._set_address(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_address(val)
                    else:
                        raise
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "view":
                try:
                    self._set_view(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_view(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(12978075317084444013, (Vertex_Reader, Vertex_Builder))


_VertexMessage_Schema = _Schema()._init(getVertexMessageSchema()).as_struct()
level0_capnp.VertexMessage.schema = _VertexMessage_Schema

cdef class VertexMessage_Reader(_DynamicStructReader):
    cdef VertexMessage.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asVertexMessage()
    

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_data(self):
        temp = self.thisptr_child.getData()
        return <bytes>((<char*>temp.begin())[:temp.size()])
        

    property data:
        def __get__(self):
            return self._get_data()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'data': _to_dict(self.data, verbose, ordered),
        
        }

        

        return ret

cdef class VertexMessage_Builder(_DynamicStructBuilder):
    cdef VertexMessage.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asVertexMessage()
    
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_data(self):
        temp = self.thisptr_child.getData()
        return <bytes>((<char*>temp.begin())[:temp.size()])
        
    cpdef _set_data(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setData(ArrayPtr[byte](<byte *>temp_string.begin(), temp_string.size()))
        

    property data:
        def __get__(self):
            return self._get_data()
        def __set__(self, value):
            self._set_data(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'data': _to_dict(self.data, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "data":
                try:
                    self._set_data(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_data(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(14406978668726422467, (VertexMessage_Reader, VertexMessage_Builder))


_DataUpdate_Schema = _Schema()._init(getDataUpdateSchema()).as_struct()
level0_capnp.DataUpdate.schema = _DataUpdate_Schema

cdef class DataUpdate_Reader(_DynamicStructReader):
    cdef DataUpdate.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asDataUpdate()
    

    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        

    property updateId:
        def __get__(self):
            return self._get_updateId()

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_mime(self):
        temp = self.thisptr_child.getMime()
        return (<char*>temp.begin())[:temp.size()]
        

    property mime:
        def __get__(self):
            return self._get_mime()

    cpdef _get_data(self):
        temp = self.thisptr_child.getData()
        return <bytes>((<char*>temp.begin())[:temp.size()])
        

    property data:
        def __get__(self):
            return self._get_data()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'mime': _to_dict(self.mime, verbose, ordered),
        
        
        'data': _to_dict(self.data, verbose, ordered),
        
        }

        

        return ret

cdef class DataUpdate_Builder(_DynamicStructBuilder):
    cdef DataUpdate.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asDataUpdate()
    
    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        
    cpdef _set_updateId(self, int64_t value):
        self.thisptr_child.setUpdateId(value)
        

    property updateId:
        def __get__(self):
            return self._get_updateId()
        def __set__(self, value):
            self._set_updateId(value)
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_mime(self):
        temp = self.thisptr_child.getMime()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_mime(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setMime(temp_string)
        

    property mime:
        def __get__(self):
            return self._get_mime()
        def __set__(self, value):
            self._set_mime(value)
    cpdef _get_data(self):
        temp = self.thisptr_child.getData()
        return <bytes>((<char*>temp.begin())[:temp.size()])
        
    cpdef _set_data(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setData(ArrayPtr[byte](<byte *>temp_string.begin(), temp_string.size()))
        

    property data:
        def __get__(self):
            return self._get_data()
        def __set__(self, value):
            self._set_data(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'mime': _to_dict(self.mime, verbose, ordered),
        
        
        'data': _to_dict(self.data, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "updateId":
                try:
                    self._set_updateId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_updateId(val)
                    else:
                        raise
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "mime":
                try:
                    self._set_mime(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_mime(val)
                    else:
                        raise
            elif key == "data":
                try:
                    self._set_data(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_data(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(15111054031566458492, (DataUpdate_Reader, DataUpdate_Builder))


_EncryptionUpdate_Schema = _Schema()._init(getEncryptionUpdateSchema()).as_struct()
level0_capnp.EncryptionUpdate.schema = _EncryptionUpdate_Schema

cdef class EncryptionUpdate_Reader(_DynamicStructReader):
    cdef EncryptionUpdate.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asEncryptionUpdate()
    

    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        

    property updateId:
        def __get__(self):
            return self._get_updateId()

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_keys(self):
        temp = self.thisptr_child.getKeys()
        return (<char*>temp.begin())[:temp.size()]
        

    property keys:
        def __get__(self):
            return self._get_keys()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'keys': _to_dict(self.keys, verbose, ordered),
        
        }

        

        return ret

cdef class EncryptionUpdate_Builder(_DynamicStructBuilder):
    cdef EncryptionUpdate.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asEncryptionUpdate()
    
    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        
    cpdef _set_updateId(self, int64_t value):
        self.thisptr_child.setUpdateId(value)
        

    property updateId:
        def __get__(self):
            return self._get_updateId()
        def __set__(self, value):
            self._set_updateId(value)
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_keys(self):
        temp = self.thisptr_child.getKeys()
        return (<char*>temp.begin())[:temp.size()]
        
    cpdef _set_keys(self, value):
        cdef StringPtr temp_string
        if type(value) is bytes:
            temp_string = StringPtr(<char*>value, len(value))
        else:
            encoded_value = value.encode('utf-8')
            temp_string = StringPtr(<char*>encoded_value, len(encoded_value))
        self.thisptr_child.setKeys(temp_string)
        

    property keys:
        def __get__(self):
            return self._get_keys()
        def __set__(self, value):
            self._set_keys(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'keys': _to_dict(self.keys, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "updateId":
                try:
                    self._set_updateId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_updateId(val)
                    else:
                        raise
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "keys":
                try:
                    self._set_keys(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_keys(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(15835948588121669718, (EncryptionUpdate_Reader, EncryptionUpdate_Builder))


_PortUpdate_Schema = _Schema()._init(getPortUpdateSchema()).as_struct()
level0_capnp.PortUpdate.schema = _PortUpdate_Schema

cdef class PortUpdate_Reader(_DynamicStructReader):
    cdef PortUpdate.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asPortUpdate()
    

    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        

    property updateId:
        def __get__(self):
            return self._get_updateId()

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_direction(self):
        return self.thisptr_child.getDirection()
        

    property direction:
        def __get__(self):
            return self._get_direction()

    cpdef _get_connectedVertex(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getConnectedVertex()
        return to_python_reader(temp, self._parent)
        

    property connectedVertex:
        def __get__(self):
            return self._get_connectedVertex()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'direction': _to_dict(self.direction, verbose, ordered),
        
        
        'connectedVertex': _to_dict(self.connectedVertex, verbose, ordered),
        
        }

        

        return ret

cdef class PortUpdate_Builder(_DynamicStructBuilder):
    cdef PortUpdate.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asPortUpdate()
    
    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        
    cpdef _set_updateId(self, int64_t value):
        self.thisptr_child.setUpdateId(value)
        

    property updateId:
        def __get__(self):
            return self._get_updateId()
        def __set__(self, value):
            self._set_updateId(value)
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_direction(self):
        return self.thisptr_child.getDirection()
        
    cpdef _set_direction(self, int64_t value):
        self.thisptr_child.setDirection(value)
        

    property direction:
        def __get__(self):
            return self._get_direction()
        def __set__(self, value):
            self._set_direction(value)
    cpdef _get_connectedVertex(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getConnectedVertex()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_connectedVertex(self, value):
        _setDynamicFieldStatic(self.thisptr, "connectedVertex", value, self._parent)
        

    property connectedVertex:
        def __get__(self):
            return self._get_connectedVertex()
        def __set__(self, value):
            self._set_connectedVertex(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'direction': _to_dict(self.direction, verbose, ordered),
        
        
        'connectedVertex': _to_dict(self.connectedVertex, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "updateId":
                try:
                    self._set_updateId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_updateId(val)
                    else:
                        raise
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "direction":
                try:
                    self._set_direction(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_direction(val)
                    else:
                        raise
            elif key == "connectedVertex":
                try:
                    self._set_connectedVertex(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_connectedVertex(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(14620046798228693937, (PortUpdate_Reader, PortUpdate_Builder))


_PortUpdate_connectedVertex_Schema = _Schema()._init(getPortUpdate_connectedVertexSchema()).as_struct()
level0_capnp.PortUpdate.ConnectedVertex.schema = _PortUpdate_connectedVertex_Schema

cdef class PortUpdate_connectedVertex_Reader(_DynamicStructReader):
    cdef PortUpdate_connectedVertex.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asPortUpdate_connectedVertex()
    

    cpdef _get_disconnected(self):
        self.thisptr_child.getDisconnected()
        return None
        

    property disconnected:
        def __get__(self):
            return self._get_disconnected()

    cpdef _get_closed(self):
        self.thisptr_child.getClosed()
        return None
        

    property closed:
        def __get__(self):
            return self._get_closed()

    cpdef _get_vertex(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertex()
        return to_python_reader(temp, self._parent)
        

    property vertex:
        def __get__(self):
            return self._get_vertex()

    cpdef _get_symlink(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getSymlink()
        return to_python_reader(temp, self._parent)
        

    property symlink:
        def __get__(self):
            return self._get_symlink()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        
        
        
        }

        
        which = self._which_str()
        ret[which] = getattr(self, which)
        

        return ret

cdef class PortUpdate_connectedVertex_Builder(_DynamicStructBuilder):
    cdef PortUpdate_connectedVertex.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asPortUpdate_connectedVertex()
    
    cpdef _get_disconnected(self):
        self.thisptr_child.getDisconnected()
        return None
        
    cpdef _set_disconnected(self, value=None):
        pass
        

    property disconnected:
        def __get__(self):
            return self._get_disconnected()
        def __set__(self, value):
            self._set_disconnected(value)
    cpdef _get_closed(self):
        self.thisptr_child.getClosed()
        return None
        
    cpdef _set_closed(self, value=None):
        pass
        

    property closed:
        def __get__(self):
            return self._get_closed()
        def __set__(self, value):
            self._set_closed(value)
    cpdef _get_vertex(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertex()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertex(self, value):
        _setDynamicFieldStatic(self.thisptr, "vertex", value, self._parent)
        

    property vertex:
        def __get__(self):
            return self._get_vertex()
        def __set__(self, value):
            self._set_vertex(value)
    cpdef _get_symlink(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getSymlink()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_symlink(self, value):
        _setDynamicFieldStatic(self.thisptr, "symlink", value, self._parent)
        

    property symlink:
        def __get__(self):
            return self._get_symlink()
        def __set__(self, value):
            self._set_symlink(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        
        
        
        }

        
        which = self._which_str()
        ret[which] = getattr(self, which)
        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "disconnected":
                try:
                    self._set_disconnected(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_disconnected(val)
                    else:
                        raise
            elif key == "closed":
                try:
                    self._set_closed(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_closed(val)
                    else:
                        raise
            elif key == "vertex":
                try:
                    self._set_vertex(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertex(val)
                    else:
                        raise
            elif key == "symlink":
                try:
                    self._set_symlink(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_symlink(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(17777914326010352767, (PortUpdate_connectedVertex_Reader, PortUpdate_connectedVertex_Builder))


_VertexState_Schema = _Schema()._init(getVertexStateSchema()).as_struct()
level0_capnp.VertexState.schema = _VertexState_Schema

cdef class VertexState_Reader(_DynamicStructReader):
    cdef VertexState.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asVertexState()
    

    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()

    cpdef _get_status(self):
        return self.thisptr_child.getStatus()
        

    property status:
        def __get__(self):
            return self._get_status()

    cpdef _get_reaped(self):
        return self.thisptr_child.getReaped()
        

    property reaped:
        def __get__(self):
            return self._get_reaped()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'status': _to_dict(self.status, verbose, ordered),
        
        
        'reaped': _to_dict(self.reaped, verbose, ordered),
        
        }

        

        return ret

cdef class VertexState_Builder(_DynamicStructBuilder):
    cdef VertexState.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asVertexState()
    
    cpdef _get_instanceId(self):
        return self.thisptr_child.getInstanceId()
        
    cpdef _set_instanceId(self, uint64_t value):
        self.thisptr_child.setInstanceId(value)
        

    property instanceId:
        def __get__(self):
            return self._get_instanceId()
        def __set__(self, value):
            self._set_instanceId(value)
    cpdef _get_status(self):
        return self.thisptr_child.getStatus()
        
    cpdef _set_status(self, uint64_t value):
        self.thisptr_child.setStatus(value)
        

    property status:
        def __get__(self):
            return self._get_status()
        def __set__(self, value):
            self._set_status(value)
    cpdef _get_reaped(self):
        return self.thisptr_child.getReaped()
        
    cpdef _set_reaped(self, cbool value):
        self.thisptr_child.setReaped(value)
        

    property reaped:
        def __get__(self):
            return self._get_reaped()
        def __set__(self, value):
            self._set_reaped(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'instanceId': _to_dict(self.instanceId, verbose, ordered),
        
        
        'status': _to_dict(self.status, verbose, ordered),
        
        
        'reaped': _to_dict(self.reaped, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "instanceId":
                try:
                    self._set_instanceId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_instanceId(val)
                    else:
                        raise
            elif key == "status":
                try:
                    self._set_status(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_status(val)
                    else:
                        raise
            elif key == "reaped":
                try:
                    self._set_reaped(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_reaped(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(14275324017576599300, (VertexState_Reader, VertexState_Builder))


_UpdateStatus_Schema = _Schema()._init(getUpdateStatusSchema()).as_struct()
level0_capnp.UpdateStatus.schema = _UpdateStatus_Schema

cdef class UpdateStatus_Reader(_DynamicStructReader):
    cdef UpdateStatus.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asUpdateStatus()
    

    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        

    property updateId:
        def __get__(self):
            return self._get_updateId()

    cpdef _get_status(self):
        return self.thisptr_child.getStatus()
        

    property status:
        def __get__(self):
            return self._get_status()

    cpdef _get_explanation(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getExplanation()
        return to_python_reader(temp, self._parent)
        

    property explanation:
        def __get__(self):
            return self._get_explanation()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'status': _to_dict(self.status, verbose, ordered),
        
        
        'explanation': _to_dict(self.explanation, verbose, ordered),
        
        }

        

        return ret

cdef class UpdateStatus_Builder(_DynamicStructBuilder):
    cdef UpdateStatus.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asUpdateStatus()
    
    cpdef _get_updateId(self):
        return self.thisptr_child.getUpdateId()
        
    cpdef _set_updateId(self, uint64_t value):
        self.thisptr_child.setUpdateId(value)
        

    property updateId:
        def __get__(self):
            return self._get_updateId()
        def __set__(self, value):
            self._set_updateId(value)
    cpdef _get_status(self):
        return self.thisptr_child.getStatus()
        
    cpdef _set_status(self, uint64_t value):
        self.thisptr_child.setStatus(value)
        

    property status:
        def __get__(self):
            return self._get_status()
        def __set__(self, value):
            self._set_status(value)
    cpdef _get_explanation(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getExplanation()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_explanation(self, value):
        _setDynamicFieldStatic(self.thisptr, "explanation", value, self._parent)
        

    property explanation:
        def __get__(self):
            return self._get_explanation()
        def __set__(self, value):
            self._set_explanation(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'updateId': _to_dict(self.updateId, verbose, ordered),
        
        
        'status': _to_dict(self.status, verbose, ordered),
        
        
        'explanation': _to_dict(self.explanation, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "updateId":
                try:
                    self._set_updateId(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_updateId(val)
                    else:
                        raise
            elif key == "status":
                try:
                    self._set_status(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_status(val)
                    else:
                        raise
            elif key == "explanation":
                try:
                    self._set_explanation(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_explanation(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(12542629989367158729, (UpdateStatus_Reader, UpdateStatus_Builder))


_ForClient_Schema = _Schema()._init(getForClientSchema()).as_struct()
level0_capnp.ForClient.schema = _ForClient_Schema

cdef class ForClient_Reader(_DynamicStructReader):
    cdef ForClient.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asForClient()
    

    cpdef _get_vertexMessages(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertexMessages()
        return to_python_reader(temp, self._parent)
        

    property vertexMessages:
        def __get__(self):
            return self._get_vertexMessages()

    cpdef _get_vertexes(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertexes()
        return to_python_reader(temp, self._parent)
        

    property vertexes:
        def __get__(self):
            return self._get_vertexes()

    cpdef _get_vertexStates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertexStates()
        return to_python_reader(temp, self._parent)
        

    property vertexStates:
        def __get__(self):
            return self._get_vertexStates()

    cpdef _get_updateStatuses(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getUpdateStatuses()
        return to_python_reader(temp, self._parent)
        

    property updateStatuses:
        def __get__(self):
            return self._get_updateStatuses()

    cpdef _get_portUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getPortUpdates()
        return to_python_reader(temp, self._parent)
        

    property portUpdates:
        def __get__(self):
            return self._get_portUpdates()

    cpdef _get_dataUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getDataUpdates()
        return to_python_reader(temp, self._parent)
        

    property dataUpdates:
        def __get__(self):
            return self._get_dataUpdates()

    cpdef _get_encryptionUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getEncryptionUpdates()
        return to_python_reader(temp, self._parent)
        

    property encryptionUpdates:
        def __get__(self):
            return self._get_encryptionUpdates()

    cpdef _get_timestamp(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getTimestamp()
        return to_python_reader(temp, self._parent)
        

    property timestamp:
        def __get__(self):
            return self._get_timestamp()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'vertexMessages': _to_dict(self.vertexMessages, verbose, ordered),
        
        
        'vertexes': _to_dict(self.vertexes, verbose, ordered),
        
        
        'vertexStates': _to_dict(self.vertexStates, verbose, ordered),
        
        
        'updateStatuses': _to_dict(self.updateStatuses, verbose, ordered),
        
        
        'portUpdates': _to_dict(self.portUpdates, verbose, ordered),
        
        
        'dataUpdates': _to_dict(self.dataUpdates, verbose, ordered),
        
        
        'encryptionUpdates': _to_dict(self.encryptionUpdates, verbose, ordered),
        
        
        'timestamp': _to_dict(self.timestamp, verbose, ordered),
        
        }

        

        return ret

cdef class ForClient_Builder(_DynamicStructBuilder):
    cdef ForClient.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asForClient()
    
    cpdef _get_vertexMessages(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertexMessages()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertexMessages(self, list value):
        cdef uint i = 0
        self.init("vertexMessages", len(value))
        cdef _DynamicListBuilder temp =  self._get_vertexMessages()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property vertexMessages:
        def __get__(self):
            return self._get_vertexMessages()
        def __set__(self, value):
            self._set_vertexMessages(value)
    cpdef _get_vertexes(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertexes()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertexes(self, list value):
        cdef uint i = 0
        self.init("vertexes", len(value))
        cdef _DynamicListBuilder temp =  self._get_vertexes()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property vertexes:
        def __get__(self):
            return self._get_vertexes()
        def __set__(self, value):
            self._set_vertexes(value)
    cpdef _get_vertexStates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertexStates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertexStates(self, list value):
        cdef uint i = 0
        self.init("vertexStates", len(value))
        cdef _DynamicListBuilder temp =  self._get_vertexStates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property vertexStates:
        def __get__(self):
            return self._get_vertexStates()
        def __set__(self, value):
            self._set_vertexStates(value)
    cpdef _get_updateStatuses(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getUpdateStatuses()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_updateStatuses(self, list value):
        cdef uint i = 0
        self.init("updateStatuses", len(value))
        cdef _DynamicListBuilder temp =  self._get_updateStatuses()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property updateStatuses:
        def __get__(self):
            return self._get_updateStatuses()
        def __set__(self, value):
            self._set_updateStatuses(value)
    cpdef _get_portUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getPortUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_portUpdates(self, list value):
        cdef uint i = 0
        self.init("portUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_portUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property portUpdates:
        def __get__(self):
            return self._get_portUpdates()
        def __set__(self, value):
            self._set_portUpdates(value)
    cpdef _get_dataUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getDataUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_dataUpdates(self, list value):
        cdef uint i = 0
        self.init("dataUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_dataUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property dataUpdates:
        def __get__(self):
            return self._get_dataUpdates()
        def __set__(self, value):
            self._set_dataUpdates(value)
    cpdef _get_encryptionUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getEncryptionUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_encryptionUpdates(self, list value):
        cdef uint i = 0
        self.init("encryptionUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_encryptionUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property encryptionUpdates:
        def __get__(self):
            return self._get_encryptionUpdates()
        def __set__(self, value):
            self._set_encryptionUpdates(value)
    cpdef _get_timestamp(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getTimestamp()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_timestamp(self, list value):
        cdef uint i = 0
        self.init("timestamp", len(value))
        cdef _DynamicListBuilder temp =  self._get_timestamp()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property timestamp:
        def __get__(self):
            return self._get_timestamp()
        def __set__(self, value):
            self._set_timestamp(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'vertexMessages': _to_dict(self.vertexMessages, verbose, ordered),
        
        
        'vertexes': _to_dict(self.vertexes, verbose, ordered),
        
        
        'vertexStates': _to_dict(self.vertexStates, verbose, ordered),
        
        
        'updateStatuses': _to_dict(self.updateStatuses, verbose, ordered),
        
        
        'portUpdates': _to_dict(self.portUpdates, verbose, ordered),
        
        
        'dataUpdates': _to_dict(self.dataUpdates, verbose, ordered),
        
        
        'encryptionUpdates': _to_dict(self.encryptionUpdates, verbose, ordered),
        
        
        'timestamp': _to_dict(self.timestamp, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "vertexMessages":
                try:
                    self._set_vertexMessages(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertexMessages(val)
                    else:
                        raise
            elif key == "vertexes":
                try:
                    self._set_vertexes(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertexes(val)
                    else:
                        raise
            elif key == "vertexStates":
                try:
                    self._set_vertexStates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertexStates(val)
                    else:
                        raise
            elif key == "updateStatuses":
                try:
                    self._set_updateStatuses(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_updateStatuses(val)
                    else:
                        raise
            elif key == "portUpdates":
                try:
                    self._set_portUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_portUpdates(val)
                    else:
                        raise
            elif key == "dataUpdates":
                try:
                    self._set_dataUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_dataUpdates(val)
                    else:
                        raise
            elif key == "encryptionUpdates":
                try:
                    self._set_encryptionUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_encryptionUpdates(val)
                    else:
                        raise
            elif key == "timestamp":
                try:
                    self._set_timestamp(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_timestamp(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(11912404238080929349, (ForClient_Reader, ForClient_Builder))


_ForService_Schema = _Schema()._init(getForServiceSchema()).as_struct()
level0_capnp.ForService.schema = _ForService_Schema

cdef class ForService_Reader(_DynamicStructReader):
    cdef ForService.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asForService()
    

    cpdef _get_vertexMessages(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getVertexMessages()
        return to_python_reader(temp, self._parent)
        

    property vertexMessages:
        def __get__(self):
            return self._get_vertexMessages()

    cpdef _get_portUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getPortUpdates()
        return to_python_reader(temp, self._parent)
        

    property portUpdates:
        def __get__(self):
            return self._get_portUpdates()

    cpdef _get_dataUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getDataUpdates()
        return to_python_reader(temp, self._parent)
        

    property dataUpdates:
        def __get__(self):
            return self._get_dataUpdates()

    cpdef _get_encryptionUpdates(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getEncryptionUpdates()
        return to_python_reader(temp, self._parent)
        

    property encryptionUpdates:
        def __get__(self):
            return self._get_encryptionUpdates()

    cpdef _get_select(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getSelect()
        return to_python_reader(temp, self._parent)
        

    property select:
        def __get__(self):
            return self._get_select()

    cpdef _get_deselect(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getDeselect()
        return to_python_reader(temp, self._parent)
        

    property deselect:
        def __get__(self):
            return self._get_deselect()

    cpdef _get_timestamp(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getTimestamp()
        return to_python_reader(temp, self._parent)
        

    property timestamp:
        def __get__(self):
            return self._get_timestamp()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'vertexMessages': _to_dict(self.vertexMessages, verbose, ordered),
        
        
        'portUpdates': _to_dict(self.portUpdates, verbose, ordered),
        
        
        'dataUpdates': _to_dict(self.dataUpdates, verbose, ordered),
        
        
        'encryptionUpdates': _to_dict(self.encryptionUpdates, verbose, ordered),
        
        
        'select': _to_dict(self.select, verbose, ordered),
        
        
        'deselect': _to_dict(self.deselect, verbose, ordered),
        
        
        'timestamp': _to_dict(self.timestamp, verbose, ordered),
        
        }

        

        return ret

cdef class ForService_Builder(_DynamicStructBuilder):
    cdef ForService.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asForService()
    
    cpdef _get_vertexMessages(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getVertexMessages()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_vertexMessages(self, list value):
        cdef uint i = 0
        self.init("vertexMessages", len(value))
        cdef _DynamicListBuilder temp =  self._get_vertexMessages()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property vertexMessages:
        def __get__(self):
            return self._get_vertexMessages()
        def __set__(self, value):
            self._set_vertexMessages(value)
    cpdef _get_portUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getPortUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_portUpdates(self, list value):
        cdef uint i = 0
        self.init("portUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_portUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property portUpdates:
        def __get__(self):
            return self._get_portUpdates()
        def __set__(self, value):
            self._set_portUpdates(value)
    cpdef _get_dataUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getDataUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_dataUpdates(self, list value):
        cdef uint i = 0
        self.init("dataUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_dataUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property dataUpdates:
        def __get__(self):
            return self._get_dataUpdates()
        def __set__(self, value):
            self._set_dataUpdates(value)
    cpdef _get_encryptionUpdates(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getEncryptionUpdates()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_encryptionUpdates(self, list value):
        cdef uint i = 0
        self.init("encryptionUpdates", len(value))
        cdef _DynamicListBuilder temp =  self._get_encryptionUpdates()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property encryptionUpdates:
        def __get__(self):
            return self._get_encryptionUpdates()
        def __set__(self, value):
            self._set_encryptionUpdates(value)
    cpdef _get_select(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getSelect()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_select(self, list value):
        cdef uint i = 0
        self.init("select", len(value))
        cdef _DynamicListBuilder temp =  self._get_select()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property select:
        def __get__(self):
            return self._get_select()
        def __set__(self, value):
            self._set_select(value)
    cpdef _get_deselect(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getDeselect()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_deselect(self, list value):
        cdef uint i = 0
        self.init("deselect", len(value))
        cdef _DynamicListBuilder temp =  self._get_deselect()
        for elem in value:
            temp[i] = elem
            i += 1
        

    property deselect:
        def __get__(self):
            return self._get_deselect()
        def __set__(self, value):
            self._set_deselect(value)
    cpdef _get_timestamp(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getTimestamp()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_timestamp(self, list value):
        cdef uint i = 0
        self.init("timestamp", len(value))
        cdef _DynamicListBuilder temp =  self._get_timestamp()
        for elem in value:
            temp._get(i).from_dict(elem)
            i += 1
        

    property timestamp:
        def __get__(self):
            return self._get_timestamp()
        def __set__(self, value):
            self._set_timestamp(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'vertexMessages': _to_dict(self.vertexMessages, verbose, ordered),
        
        
        'portUpdates': _to_dict(self.portUpdates, verbose, ordered),
        
        
        'dataUpdates': _to_dict(self.dataUpdates, verbose, ordered),
        
        
        'encryptionUpdates': _to_dict(self.encryptionUpdates, verbose, ordered),
        
        
        'select': _to_dict(self.select, verbose, ordered),
        
        
        'deselect': _to_dict(self.deselect, verbose, ordered),
        
        
        'timestamp': _to_dict(self.timestamp, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "vertexMessages":
                try:
                    self._set_vertexMessages(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_vertexMessages(val)
                    else:
                        raise
            elif key == "portUpdates":
                try:
                    self._set_portUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_portUpdates(val)
                    else:
                        raise
            elif key == "dataUpdates":
                try:
                    self._set_dataUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_dataUpdates(val)
                    else:
                        raise
            elif key == "encryptionUpdates":
                try:
                    self._set_encryptionUpdates(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_encryptionUpdates(val)
                    else:
                        raise
            elif key == "select":
                try:
                    self._set_select(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_select(val)
                    else:
                        raise
            elif key == "deselect":
                try:
                    self._set_deselect(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_deselect(val)
                    else:
                        raise
            elif key == "timestamp":
                try:
                    self._set_timestamp(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_timestamp(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(15292146655510213443, (ForService_Reader, ForService_Builder))


_Message_Schema = _Schema()._init(getMessageSchema()).as_struct()
level0_capnp.Message.schema = _Message_Schema

cdef class Message_Reader(_DynamicStructReader):
    cdef Message.Reader thisptr_child
    def __init__(self, _DynamicStructReader struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Reader>struct.thisptr).asMessage()
    

    cpdef _get_forClient(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getForClient()
        return to_python_reader(temp, self._parent)
        

    property forClient:
        def __get__(self):
            return self._get_forClient()

    cpdef _get_forService(self):
        cdef DynamicValue.Reader temp = self.thisptr_child.getForService()
        return to_python_reader(temp, self._parent)
        

    property forService:
        def __get__(self):
            return self._get_forService()

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'forClient': _to_dict(self.forClient, verbose, ordered),
        
        
        'forService': _to_dict(self.forService, verbose, ordered),
        
        }

        

        return ret

cdef class Message_Builder(_DynamicStructBuilder):
    cdef Message.Builder thisptr_child
    def __init__(self, _DynamicStructBuilder struct):
        self._init(struct.thisptr, struct._parent, struct.is_root, False)
        self.thisptr_child = (<C_DynamicStruct_Builder>struct.thisptr).asMessage()
    
    cpdef _get_forClient(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getForClient()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_forClient(self, value):
        _setDynamicFieldStatic(self.thisptr, "forClient", value, self._parent)
        

    property forClient:
        def __get__(self):
            return self._get_forClient()
        def __set__(self, value):
            self._set_forClient(value)
    cpdef _get_forService(self):
        cdef DynamicValue.Builder temp = self.thisptr_child.getForService()
        return to_python_builder(temp, self._parent)
        
    cpdef _set_forService(self, value):
        _setDynamicFieldStatic(self.thisptr, "forService", value, self._parent)
        

    property forService:
        def __get__(self):
            return self._get_forService()
        def __set__(self, value):
            self._set_forService(value)

    def to_dict(self, verbose=False, ordered=False):
        ret = {
        
        
        'forClient': _to_dict(self.forClient, verbose, ordered),
        
        
        'forService': _to_dict(self.forService, verbose, ordered),
        
        }

        

        return ret

    def from_dict(self, dict d):
        cdef str key
        for key, val in d.iteritems():
            if False: pass
        
            elif key == "forClient":
                try:
                    self._set_forClient(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_forClient(val)
                    else:
                        raise
            elif key == "forService":
                try:
                    self._set_forService(val)
                except Exception as e:
                    if 'expected isSetInUnion(field)' in str(e):
                        self.init(key)
                        self._set_forService(val)
                    else:
                        raise
            else:
                raise ValueError('Key not found in struct: ' + key)


capnp.register_type(9608355894647239841, (Message_Reader, Message_Builder))
