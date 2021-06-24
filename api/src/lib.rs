extern crate jni;

use algebra::{SemanticallyValid, serialize::*};
use demo_circuit::{type_mapping::*, get_instance_for_setup};
use cctp_primitives::utils::{
    serialization::*,
    poseidon_hash::*,
    mht::*,
    proof_system::*,
};
use std::any::type_name;

mod ginger_calls;
use ginger_calls::*;

fn read_raw_pointer<'a, T>(input: *const T) -> &'a T {
    assert!(!input.is_null());
    unsafe { &*input }
}

fn read_mut_raw_pointer<'a, T>(input: *mut T) -> &'a mut T {
    assert!(!input.is_null());
    unsafe { &mut *input }
}

fn read_nullable_raw_pointer<'a, T>(input: *const T) -> Option<&'a T> {
    unsafe { input.as_ref() }
}

fn serialize_from_raw_pointer<T: CanonicalSerialize>(
    to_write: *const T,
) -> Vec<u8> {
    serialize_to_buffer(read_raw_pointer(to_write))
        .expect(format!("unable to write {} to buffer", type_name::<T>()).as_str())
}

fn return_jobject<'a, T: Sized>(_env: &'a JNIEnv, obj: T, class_path: &str) -> JObject<'a>
{
    //Return field element
    let obj_ptr: jlong = jlong::from(Box::into_raw(Box::new(obj)) as i64);

    let obj_class = _env.find_class(class_path).expect("Should be able to find class");

    _env.new_object(obj_class, "(J)V", &[JValue::Long(obj_ptr)])
        .expect("Should be able to create new jobject")
}

fn deserialize_to_jobject<T: CanonicalDeserialize + SemanticallyValid>(
    _env: &JNIEnv,
    obj_bytes: jbyteArray,
    checked: jboolean,
    class_path: &str
) -> jobject
{
    let obj_bytes = _env.convert_byte_array(obj_bytes)
        .expect("Cannot read bytes.");

    let obj = if checked == JNI_TRUE {
        deserialize_from_buffer_checked::<T>(obj_bytes.as_slice())
    } else {
        deserialize_from_buffer::<T>(obj_bytes.as_slice())
    };

    match obj {
        Ok(obj) => *return_jobject(&_env, obj, class_path),
        Err(_) => std::ptr::null::<jobject>() as jobject,
    }
}

fn serialize_from_jobject<T: CanonicalSerialize>(
    _env: &JNIEnv,
    obj: JObject,
    ptr_name: &str
) -> jbyteArray
{
    let pointer = _env.get_field(obj, ptr_name, "J")
        .expect("Cannot get object raw pointer.");

    let obj = read_raw_pointer(pointer.j().unwrap() as *const T);

    let obj_bytes = serialize_from_raw_pointer(obj);

    _env.byte_array_from_slice(obj_bytes.as_slice())
        .expect("Cannot write object.")
}

use jni::JNIEnv;
use jni::objects::{JClass, JString, JObject, JValue};
use jni::sys::{jbyteArray, jboolean, jint, jlong, /*jlongArray, */jobject, jobjectArray};
use jni::sys::{JNI_TRUE, JNI_FALSE};

//Field element related functions

fn return_field_element(_env: &JNIEnv, fe: FieldElement) -> jobject
{
    return_jobject(_env, fe, "com/horizen/librustsidechains/FieldElement")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeGetFieldElementSize(
    _env: JNIEnv,
    _field_element_class: JClass,
) -> jint { FIELD_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeSerializeFieldElement(
    _env: JNIEnv,
    _field_element: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<FieldElement>(&_env, _field_element, "fieldElementPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeDeserializeFieldElement(
    _env: JNIEnv,
    _class: JClass,
    _field_element_bytes: jbyteArray,
) -> jobject
{
    deserialize_to_jobject::<FieldElement>(
        &_env,
        _field_element_bytes,
        JNI_FALSE,
        "com/horizen/librustsidechains/FieldElement"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeCreateRandom(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _seed: jlong,
) -> jobject
{
    //Create random field element
    let fe = get_random_field_element(_seed as u64);

    return_field_element(&_env, fe)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeCreateFromLong(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _long: jlong
) -> jobject
{
    //Create field element from _long
    let fe = FieldElement::from(_long as u64);

    return_field_element(&_env, fe)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativePrintFieldElementBytes(
    _env: JNIEnv,
    _field_element: JObject,
)
{
    let pointer = _env.get_field(_field_element, "fieldElementPointer", "J")
        .expect("Cannot get object raw pointer.");

    let obj = read_raw_pointer(pointer.j().unwrap() as *const FieldElement);

    let obj_bytes = serialize_from_raw_pointer(obj);

    println!("{:?}", into_i8(obj_bytes));
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeFreeFieldElement(
    _env: JNIEnv,
    _class: JClass,
    _fe: *mut FieldElement,
)
{
    if _fe.is_null()  { return }
    drop(unsafe { Box::from_raw(_fe) });
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_librustsidechains_FieldElement_nativeEquals(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _field_element_1: JObject,
    _field_element_2: JObject,
) -> jboolean
{
    //Read field_1
    let field_1 = {

        let f =_env.get_field(_field_element_1, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer_1");

        read_raw_pointer(f.j().unwrap() as *const FieldElement)
    };

    //Read field_2
    let field_2 = {

        let f =_env.get_field(_field_element_2, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer_2");

        read_raw_pointer(f.j().unwrap() as *const FieldElement)
    };

    match field_1 == field_2 {
        true => JNI_TRUE,
        false => JNI_FALSE,
    }
}

//Public Schnorr key utility functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeGetPublicKeySize(
    _env: JNIEnv,
    _schnorr_public_key_class: JClass,
) -> jint { SCHNORR_PK_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeSerializePublicKey(
    _env: JNIEnv,
    _schnorr_public_key: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<SchnorrPk>(&_env, _schnorr_public_key, "publicKeyPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeDeserializePublicKey(
    _env: JNIEnv,
    _schnorr_public_key_class: JClass,
    _public_key_bytes: jbyteArray,
    _check_public_key: jboolean,
) -> jobject
{
    deserialize_to_jobject::<SchnorrPk>(
        &_env,
        _public_key_bytes,
        _check_public_key,
        "com/horizen/schnorrnative/SchnorrPublicKey"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeFreePublicKey(
    _env: JNIEnv,
    _schnorr_public_key: JObject,
)
{
    let public_key_pointer = _env.get_field(_schnorr_public_key, "publicKeyPointer", "J")
        .expect("Cannot get public key pointer.");

    let public_key = public_key_pointer.j().unwrap() as *mut SchnorrPk;

    if public_key.is_null()  { return }
    drop(unsafe { Box::from_raw(public_key) });
}

//Secret Schnorr key utility functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSecretKey_nativeGetSecretKeySize(
    _env: JNIEnv,
    _schnorr_secret_key_class: JClass,
) -> jint { SCHNORR_SK_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSecretKey_nativeSerializeSecretKey(
    _env: JNIEnv,
    _schnorr_secret_key: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<SchnorrSk>(&_env, _schnorr_secret_key, "secretKeyPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSecretKey_nativeDeserializeSecretKey(
    _env: JNIEnv,
    _schnorr_secret_key_class: JClass,
    _secret_key_bytes: jbyteArray,
) -> jobject
{
    deserialize_to_jobject::<SchnorrSk>(
        &_env,
        _secret_key_bytes,
        JNI_FALSE,
        "com/horizen/schnorrnative/SchnorrSecretKey"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSecretKey_nativeFreeSecretKey(
    _env: JNIEnv,
    _schnorr_secret_key: JObject,
)
{
    let secret_key_pointer = _env.get_field(_schnorr_secret_key, "secretKeyPointer", "J")
        .expect("Cannot get secret key pointer.");

    let secret_key = secret_key_pointer.j().unwrap() as *mut SchnorrSk;

    if secret_key.is_null()  { return }
    drop(unsafe { Box::from_raw(secret_key) });
}

//Public VRF key utility functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeGetPublicKeySize(
    _env: JNIEnv,
    _vrf_public_key_class: JClass,
) -> jint { VRF_PK_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeSerializePublicKey(
    _env: JNIEnv,
    _vrf_public_key: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<VRFPk>(&_env, _vrf_public_key, "publicKeyPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeDeserializePublicKey(
    _env: JNIEnv,
    _vrf_public_key_class: JClass,
    _public_key_bytes: jbyteArray,
    _check_public_key: jboolean,
) -> jobject
{
    deserialize_to_jobject::<VRFPk>(
        &_env,
        _public_key_bytes,
        _check_public_key,
        "com/horizen/vrfnative/VRFPublicKey"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeFreePublicKey(
    _env: JNIEnv,
    _vrf_public_key: JObject,
)
{
    let public_key_pointer = _env.get_field(_vrf_public_key, "publicKeyPointer", "J")
        .expect("Cannot get public key pointer.");

    let public_key = public_key_pointer.j().unwrap() as *mut SchnorrPk;

    if public_key.is_null()  { return }
    drop(unsafe { Box::from_raw(public_key) });
}

//Secret VRF key utility functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFSecretKey_nativeGetSecretKeySize(
    _env: JNIEnv,
    _vrf_secret_key_class: JClass,
) -> jint { VRF_SK_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFSecretKey_nativeSerializeSecretKey(
    _env: JNIEnv,
    _vrf_secret_key: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<VRFSk>(&_env, _vrf_secret_key, "secretKeyPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFSecretKey_nativeDeserializeSecretKey(
    _env: JNIEnv,
    _vrf_secret_key_class: JClass,
    _secret_key_bytes: jbyteArray,
) -> jobject
{
    deserialize_to_jobject::<VRFSk>(
        &_env,
        _secret_key_bytes,
        JNI_FALSE,
        "com/horizen/vrfnative/VRFSecretKey"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFSecretKey_nativeFreeSecretKey(
    _env: JNIEnv,
    _vrf_secret_key: JObject,
)
{
    let secret_key_pointer = _env.get_field(_vrf_secret_key, "secretKeyPointer", "J")
        .expect("Cannot get secret key pointer.");

    let secret_key = secret_key_pointer.j().unwrap() as *mut SchnorrSk;

    if secret_key.is_null()  { return }
    drop(unsafe { Box::from_raw(secret_key) });
}

//Schnorr signature utility functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSignature_nativeGetSignatureSize(
    _env: JNIEnv,
    _class: JClass,
) -> jint { SCHNORR_SIG_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSignature_nativeSerializeSignature(
    _env: JNIEnv,
    _schnorr_sig: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<SchnorrSig>(&_env, _schnorr_sig, "signaturePointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSignature_nativeDeserializeSignature(
    _env: JNIEnv,
    _class: JClass,
    _sig_bytes: jbyteArray,
    _check_sig: jboolean,
) -> jobject
{
    deserialize_to_jobject::<SchnorrSig>(
        &_env,
        _sig_bytes,
        _check_sig,
        "com/horizen/schnorrnative/SchnorrSignature"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSignature_nativeIsValidSignature(
    _env: JNIEnv,
    _sig: JObject,
) -> jboolean
{
    let sig = _env.get_field(_sig, "signaturePointer", "J")
        .expect("Should be able to get field signaturePointer").j().unwrap() as *const SchnorrSig;

    if is_valid(read_raw_pointer(sig)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSignature_nativefreeSignature(
    _env: JNIEnv,
    _class: JClass,
    _sig: *mut SchnorrSig,
)
{
    if _sig.is_null()  { return }
    drop(unsafe { Box::from_raw(_sig) });
}

//Schnorr signature functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrKeyPair_nativeGenerate(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
) -> jobject
{
    let (pk, sk) = schnorr_generate_key();

    let secret_key_object = return_jobject(&_env, sk, "com/horizen/schnorrnative/SchnorrSecretKey");
    let public_key_object = return_jobject(&_env, pk, "com/horizen/schnorrnative/SchnorrPublicKey");

    let class = _env.find_class("com/horizen/schnorrnative/SchnorrKeyPair")
        .expect("Should be able to find SchnorrKeyPair class");

    let result = _env.new_object(
        class,
        "(Lcom/horizen/schnorrnative/SchnorrSecretKey;Lcom/horizen/schnorrnative/SchnorrPublicKey;)V",
        &[JValue::Object(secret_key_object), JValue::Object(public_key_object)]
    ).expect("Should be able to create new (SchnorrSecretKey, SchnorrPublicKey) object");

    *result
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrKeyPair_nativeSignMessage(
    _env: JNIEnv,
    _schnorr_key_pair: JObject,
    _message: JObject,
) -> jobject {

    //Read sk
    let sk_object = _env.get_field(_schnorr_key_pair,
                                   "secretKey",
                                   "Lcom/horizen/schnorrnative/SchnorrSecretKey;"
    ).expect("Should be able to get field secretKey").l().unwrap();
    let secret_key = {

        let s =_env.get_field(sk_object, "secretKeyPointer", "J")
            .expect("Should be able to get field secretKeyPointer");

        read_raw_pointer(s.j().unwrap() as *const SchnorrSk)
    };

    //Read pk
    let pk_object = _env.get_field(_schnorr_key_pair,
                                   "publicKey",
                                   "Lcom/horizen/schnorrnative/SchnorrPublicKey;"
    ).expect("Should be able to get field publicKey").l().unwrap();

    let public_key = {

        let p = _env.get_field(pk_object, "publicKeyPointer", "J")
            .expect("Should be able to get field publicKeyPointer");

        read_raw_pointer(p.j().unwrap() as *const SchnorrPk)
    };

    //Read message
    let message = {

        let m =_env.get_field(_message, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(m.j().unwrap() as *const FieldElement)
    };

    //Sign message and return opaque pointer to sig
    let signature = match schnorr_sign(message, secret_key, public_key) {
        Ok(sig) => sig,
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    };

    return_jobject(&_env, signature, "com/horizen/schnorrnative/SchnorrSignature")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeVerifyKey(
    _env: JNIEnv,
    _public_key: JObject,
) -> jboolean
{
    let pk = _env.get_field(_public_key, "publicKeyPointer", "J")
        .expect("Should be able to get field publicKeyPointer").j().unwrap() as *const SchnorrPk;

    if schnorr_verify_public_key(read_raw_pointer(pk)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrSecretKey_nativeGetPublicKey(
    _env: JNIEnv,
    _secret_key: JObject
) -> jobject {

    let sk = _env.get_field(_secret_key, "secretKeyPointer", "J")
        .expect("Should be able to get field secretKeyPointer").j().unwrap() as *const SchnorrSk;

    let secret_key = read_raw_pointer(sk);

    let pk = schnorr_get_public_key(secret_key);

    return_jobject(&_env, pk, "com/horizen/schnorrnative/SchnorrPublicKey")
        .into_inner()
}


#[no_mangle]
pub extern "system" fn Java_com_horizen_schnorrnative_SchnorrPublicKey_nativeVerifySignature(
    _env: JNIEnv,
    _public_key: JObject,
    _signature: JObject,
    _message: JObject,
) -> jboolean {

    //Read pk
    let public_key = {

        let p = _env.get_field(_public_key, "publicKeyPointer", "J")
            .expect("Should be able to get field publicKeyPointer");

        read_raw_pointer(p.j().unwrap() as *const SchnorrPk)
    };

    //Read message
    let message = {

        let m =_env.get_field(_message, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(m.j().unwrap() as *const FieldElement)
    };

    //Read sig
    let signature = {
        let sig = _env.get_field(_signature, "signaturePointer", "J")
            .expect("Should be able to get field signaturePointer");

        read_raw_pointer(sig.j().unwrap() as *const SchnorrSig)
    };

    //Verify sig
    match schnorr_verify_signature(message, public_key, signature) {
        Ok(result) => if result {
            JNI_TRUE
        } else {
            JNI_FALSE
        },
        Err(_) => JNI_FALSE //CRYPTO_ERROR
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeGetHashSize(
    _env: JNIEnv,
    _class: JClass,
) -> jint { FIELD_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeGetConstantLengthPoseidonHash(
    _env: JNIEnv,
    _class: JClass,
    _input_size: jint,
    _personalization: jobjectArray,
) -> jobject
{
    //Read _personalization as array of FieldElement
    let personalization_len = _env.get_array_length(_personalization)
        .expect("Should be able to read personalization array size");
    let mut personalization = vec![];

    // Array can be empty
    for i in 0..personalization_len {
        let field_obj = _env.get_object_array_element(_personalization, i)
            .expect(format!("Should be able to read elem {} of the personalization array", i).as_str());

        let field = {

            let f =_env.get_field(field_obj, "fieldElementPointer", "J")
                .expect("Should be able to get field fieldElementPointer");

            read_raw_pointer(f.j().unwrap() as *const FieldElement)
        };

        personalization.push(*field);
    }

    //Instantiate PoseidonHash
    let h = get_poseidon_hash_constant_length(
        _input_size as usize,
        if personalization.is_empty() { None } else { Some(personalization.as_slice()) }
    );

    //Return PoseidonHash instance
    return_jobject(&_env, h, "com/horizen/poseidonnative/PoseidonHash")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeGetVariableLengthPoseidonHash(
    _env: JNIEnv,
    _class: JClass,
    _mod_rate: jboolean,
    _personalization: jobjectArray,
) -> jobject
{
    //Read _personalization as array of FieldElement
    let personalization_len = _env.get_array_length(_personalization)
        .expect("Should be able to read personalization array size");
    let mut personalization = vec![];

    // Array can be empty
    for i in 0..personalization_len {
        let field_obj = _env.get_object_array_element(_personalization, i)
            .expect(format!("Should be able to read elem {} of the personalization array", i).as_str());

        let field = {

            let f =_env.get_field(field_obj, "fieldElementPointer", "J")
                .expect("Should be able to get field fieldElementPointer");

            read_raw_pointer(f.j().unwrap() as *const FieldElement)
        };

        personalization.push(*field);
    }

    //Instantiate PoseidonHash
    let h = get_poseidon_hash_variable_length(
        _mod_rate == JNI_TRUE,
        if personalization.is_empty() { None } else { Some(personalization.as_slice()) }
    );

    //Return PoseidonHash instance
    return_jobject(&_env, h, "com/horizen/poseidonnative/PoseidonHash")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeUpdate(
    _env: JNIEnv,
    _h: JObject,
    _input: JObject,
){
    //Read PoseidonHash instance
    let digest = {

        let h = _env.get_field(_h, "poseidonHashPointer", "J")
            .expect("Should be able to get field poseidonHashPointer");

        read_mut_raw_pointer(h.j().unwrap() as *mut FieldHash)
    };

    //Read input
    let input = {

        let i =_env.get_field(_input, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(i.j().unwrap() as *const FieldElement)
    };

    update_poseidon_hash(digest, input);
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeFinalize(
    _env: JNIEnv,
    _h: JObject,
) -> jobject
{
    //Read PoseidonHash instance
    let digest = {

        let h = _env.get_field(_h, "poseidonHashPointer", "J")
            .expect("Should be able to get field poseidonHashPointer");

        read_raw_pointer(h.j().unwrap() as *const FieldHash)
    };

    //Get digest
    let fe = match finalize_poseidon_hash(digest) {
        Ok(fe) => fe,
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    };

    return_field_element(&_env, fe)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeReset(
    _env: JNIEnv,
    _h: JObject,
    _personalization: jobjectArray,
){
    //Read PoseidonHash instance
    let digest = {

        let h = _env.get_field(_h, "poseidonHashPointer", "J")
            .expect("Should be able to get field poseidonHashPointer");

        read_mut_raw_pointer(h.j().unwrap() as *mut FieldHash)
    };

    //Read _personalization as array of FieldElement
    let personalization_len = _env.get_array_length(_personalization)
        .expect("Should be able to read personalization array size");
    let mut personalization = vec![];

    // Array can be empty
    for i in 0..personalization_len {
        let field_obj = _env.get_object_array_element(_personalization, i)
            .expect(format!("Should be able to read elem {} of the personalization array", i).as_str());

        let field = {

            let f =_env.get_field(field_obj, "fieldElementPointer", "J")
                .expect("Should be able to get field fieldElementPointer");

            read_raw_pointer(f.j().unwrap() as *const FieldElement)
        };

        personalization.push(*field);
    }

    let personalization = if personalization.is_empty() { None } else { Some(personalization.as_slice()) };

    reset_poseidon_hash(digest, personalization)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_poseidonnative_PoseidonHash_nativeFreePoseidonHash(
    _env: JNIEnv,
    _h: JObject,
)
{
    let h_pointer = _env.get_field(_h, "poseidonHashPointer", "J")
        .expect("Cannot get poseidonHashPointer");

    let h = h_pointer.j().unwrap() as *mut FieldHash;

    if h.is_null()  { return }
    drop(unsafe { Box::from_raw(h) });
}

//Merkle tree functions

//////////// MERKLE PATH

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeVerify(
    _env: JNIEnv,
    _path: JObject,
    _height: jint,
    _leaf: JObject,
    _root: JObject,
) -> jboolean
{
    let leaf = {

        let fe =_env.get_field(_leaf, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let root = {

        let fe =_env.get_field(_root, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    match verify_ginger_merkle_path(path, _height as usize, leaf, root) {
        Ok(result) => if result { JNI_TRUE } else { JNI_FALSE },
        Err(_) => JNI_FALSE // CRYPTO_ERROR
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeVerifyWithoutLengthCheck(
    _env: JNIEnv,
    _path: JObject,
    _leaf: JObject,
    _root: JObject,
) -> jboolean
{
    let leaf = {

        let fe =_env.get_field(_leaf, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let root = {

        let fe =_env.get_field(_root, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    if verify_ginger_merkle_path_without_length_check(path, leaf, root) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeApply(
    _env: JNIEnv,
    _path: JObject,
    _leaf: JObject,
) -> jobject
{
    let path = {
        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    let leaf = {

        let fe =_env.get_field(_leaf, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let root = get_root_from_path(path, leaf);

    return_field_element(&_env, root)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeIsLeftmost(
    _env: JNIEnv,
    _path: JObject,
) -> jboolean
{
    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    is_path_leftmost(path) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeIsRightmost(
    _env: JNIEnv,
    _path: JObject,
) -> jboolean
{
    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    is_path_rightmost(path) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeAreRightLeavesEmpty(
    _env: JNIEnv,
    _path: JObject,
) -> jboolean
{
    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    are_right_leaves_empty(path) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeLeafIndex(
    _env: JNIEnv,
    _path: JObject,
) -> jlong
{
    let path = {

        let t =_env.get_field(_path, "merklePathPointer", "J")
            .expect("Should be able to get field merklePathPointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHTPath)
    };

    get_leaf_index_from_path(path) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeSerialize(
    _env: JNIEnv,
    _path: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<GingerMHTPath>(&_env, _path, "merklePathPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeDeserialize(
    _env: JNIEnv,
    _class: JClass,
    _path_bytes: jbyteArray,
) -> jobject
{
    deserialize_to_jobject::<GingerMHTPath>(
        &_env,
        _path_bytes,
        JNI_FALSE,
        "com/horizen/merkletreenative/MerklePath"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_MerklePath_nativeFreeMerklePath(
    _env: JNIEnv,
    _class: JClass,
    _path: *mut GingerMHTPath,
)
{
    if _path.is_null()  { return }
    drop(unsafe { Box::from_raw(_path) });
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeInit(
    _env: JNIEnv,
    _class: JClass,
    _height: jint,
    _processing_step: jlong,
) -> jobject
{
    // Create new InMemoryOptimizedMerkleTree Rust side
    let mt = new_ginger_mht(
        _height as usize,
        _processing_step as usize
    );

    // Create and return new InMemoryOptimizedMerkleTree Java side
    return_jobject(&_env, mt, "com/horizen/merkletreenative/InMemoryOptimizedMerkleTree")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeAppend(
    _env: JNIEnv,
    _tree: JObject,
    _leaf: JObject,
) -> jboolean
{
    let leaf = {

        let fe =_env.get_field(_leaf, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(fe.j().unwrap() as *const FieldElement)
    };

    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_mut_raw_pointer(t.j().unwrap() as *mut GingerMHT)
    };

    match append_leaf_to_ginger_mht(tree, leaf) {
        Ok(_) => JNI_TRUE,
        Err(_) => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeFinalize(
    _env: JNIEnv,
    _tree: JObject,
) -> jobject
{
    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHT)
    };

    let tree_copy = finalize_ginger_mht(tree);

    return_jobject(&_env, tree_copy, "com/horizen/merkletreenative/InMemoryOptimizedMerkleTree")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeFinalizeInPlace(
    _env: JNIEnv,
    _tree: JObject,
)
{
    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_mut_raw_pointer(t.j().unwrap() as *mut GingerMHT)
    };

    finalize_ginger_mht_in_place(tree);
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeRoot(
    _env: JNIEnv,
    _tree: JObject,
) -> jobject
{
    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHT)
    };

    let root = get_ginger_mht_root(tree)
        .expect("Tree must've been finalized");

    return_field_element(&_env, root)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeGetMerklePath(
    _env: JNIEnv,
    _tree: JObject,
    _leaf_index: jlong,
) -> jobject
{
    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_raw_pointer(t.j().unwrap() as *const GingerMHT)
    };

    let path = get_ginger_mht_path(tree, _leaf_index as u64)
        .expect("Tree must've been finalized");

    return_jobject(&_env, path, "com/horizen/merkletreenative/MerklePath")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeReset(
    _env: JNIEnv,
    _tree: JObject,
)
{
    let tree = {

        let t =_env.get_field(_tree, "inMemoryOptimizedMerkleTreePointer", "J")
            .expect("Should be able to get field inMemoryOptimizedMerkleTreePointer");

        read_mut_raw_pointer(t.j().unwrap() as *mut GingerMHT)
    };

    reset_ginger_mht(tree);
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_merkletreenative_InMemoryOptimizedMerkleTree_nativeFreeInMemoryOptimizedMerkleTree(
    _env: JNIEnv,
    _class: JClass,
    _tree: *mut GingerMHT,
)
{
    if _tree.is_null()  { return }
    drop(unsafe { Box::from_raw(_tree) });
}

//VRF utility functions

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFProof_nativeGetProofSize(
    _env: JNIEnv,
    _class: JClass,
) -> jint { VRF_PROOF_SIZE as jint }

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFProof_nativeSerializeProof(
    _env: JNIEnv,
    _proof: JObject,
) -> jbyteArray
{
    serialize_from_jobject::<VRFProof>(&_env, _proof, "proofPointer")
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFProof_nativeDeserializeProof(
    _env: JNIEnv,
    _class: JClass,
    _proof_bytes: jbyteArray,
    _check_proof: jboolean,
) -> jobject
{
    deserialize_to_jobject::<VRFProof>(
        &_env,
        _proof_bytes,
        _check_proof,
        "com/horizen/vrfnative/VRFProof"
    )
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFProof_nativeIsValidVRFProof(
    _env: JNIEnv,
    _vrf_proof: JObject,
) -> jboolean
{
    let proof = _env.get_field(_vrf_proof, "proofPointer", "J")
        .expect("Should be able to get field proofPointer").j().unwrap() as *const VRFProof;

    if is_valid(read_raw_pointer(proof)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFProof_nativefreeProof(
    _env: JNIEnv,
    _class: JClass,
    _proof: *mut VRFProof,
)
{
    if _proof.is_null()  { return }
    drop(unsafe { Box::from_raw(_proof) });
}


//VRF functions
#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFKeyPair_nativeGenerate(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass
) -> jobject
{

    let (pk, sk) = vrf_generate_key();

    let secret_key_object = return_jobject(&_env, sk, "com/horizen/vrfnative/VRFSecretKey");
    let public_key_object = return_jobject(&_env, pk, "com/horizen/vrfnative/VRFPublicKey");

    let class = _env.find_class("com/horizen/vrfnative/VRFKeyPair")
        .expect("Should be able to find VRFKeyPair class");

    let result = _env.new_object(
        class,
        "(Lcom/horizen/vrfnative/VRFSecretKey;Lcom/horizen/vrfnative/VRFPublicKey;)V",
        &[JValue::Object(secret_key_object), JValue::Object(public_key_object)]
    ).expect("Should be able to create new (VRFSecretKey, VRFPublicKey) object");

    *result
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFKeyPair_nativeProve(
    _env: JNIEnv,
    _vrf_key_pair: JObject,
    _message: JObject
) -> jobject {

    //Read sk
    let sk_object = _env.get_field(_vrf_key_pair,
                                   "secretKey",
                                   "Lcom/horizen/vrfnative/VRFSecretKey;"
    ).expect("Should be able to get field vrfKey").l().unwrap();

    let secret_key = {

        let s =_env.get_field(sk_object, "secretKeyPointer", "J")
            .expect("Should be able to get field secretKeyPointer");

        read_raw_pointer(s.j().unwrap() as *const VRFSk)
    };

    //Read pk
    let pk_object = _env.get_field(_vrf_key_pair,
                                   "publicKey",
                                   "Lcom/horizen/vrfnative/VRFPublicKey;"
    ).expect("Should be able to get field publicKey").l().unwrap();

    let public_key = {

        let p = _env.get_field(pk_object, "publicKeyPointer", "J")
            .expect("Should be able to get field publicKeyPointer");

        read_raw_pointer(p.j().unwrap() as *const VRFPk)
    };

    //Read message
    let message = {

        let m =_env.get_field(_message, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(m.j().unwrap() as *const FieldElement)
    };

    //Compute vrf proof
    let (proof, vrf_out) = match vrf_prove(message, secret_key, public_key) {
        Ok((proof, vrf_out)) => (
            return_jobject(&_env, proof, "com/horizen/vrfnative/VRFProof"),
            return_jobject(&_env, vrf_out, "com/horizen/librustsidechains/FieldElement")
        ),
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    };

    //Create and return VRFProveResult instance
    let class = _env.find_class("com/horizen/vrfnative/VRFProveResult")
        .expect("Should be able to find VRFProveResult class");

    let result = _env.new_object(
        class,
        "(Lcom/horizen/vrfnative/VRFProof;Lcom/horizen/librustsidechains/FieldElement;)V",
        &[JValue::Object(proof), JValue::Object(vrf_out)]
    ).expect("Should be able to create new VRFProveResult:(VRFProof, FieldElement) object");

    *result
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFSecretKey_nativeGetPublicKey(
    _env: JNIEnv,
    _vrf_secret_key: JObject
) -> jobject {

    let sk = _env.get_field(_vrf_secret_key, "secretKeyPointer", "J")
        .expect("Should be able to get field secretKeyPointer").j().unwrap() as *const VRFSk;

    let secret_key = read_raw_pointer(sk);

    let pk = vrf_get_public_key(secret_key);
    return_jobject(&_env, pk, "com/horizen/vrfnative/VRFPublicKey").into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeVerifyKey(
    _env: JNIEnv,
    _vrf_public_key: JObject,
) -> jboolean
{
    let pk = _env.get_field(_vrf_public_key, "publicKeyPointer", "J")
        .expect("Should be able to get field publicKeyPointer").j().unwrap() as *const VRFPk;

    if vrf_verify_public_key(read_raw_pointer(pk)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_vrfnative_VRFPublicKey_nativeProofToHash(
    _env: JNIEnv,
    _vrf_public_key: JObject,
    _proof: JObject,
    _message: JObject,
) -> jobject
{
    let public_key = {

        let p = _env.get_field(_vrf_public_key, "publicKeyPointer", "J")
            .expect("Should be able to get field publicKeyPointer");

        read_raw_pointer(p.j().unwrap() as *const VRFPk)
    };

    //Read message
    let message = {

        let m =_env.get_field(_message, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(m.j().unwrap() as *const FieldElement)
    };

    //Read proof
    let proof = {
        let p = _env.get_field(_proof, "proofPointer", "J")
            .expect("Should be able to get field proofPointer");

        read_raw_pointer(p.j().unwrap() as *const VRFProof)
    };

    //Verify vrf proof and get vrf output
    let vrf_out = match vrf_proof_to_hash(message, public_key, proof) {
        Ok(result) => result,
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    };

    //Return vrf output
    return_field_element(&_env, vrf_out)
}

//Naive threshold signature proof functions

#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_BackwardTransfer_nativeGetMcPkHashSize(
    _env: JNIEnv,
    _class: JClass,
) -> jint
{
    MC_PK_SIZE as jint
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_NaiveThresholdSigProof_nativeGetConstant(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _schnorr_pks_list: jobjectArray,
    _threshold: jlong,
) -> jobject
{
    //Extract Schnorr pks
    let mut pks = vec![];

    let pks_list_size = _env.get_array_length(_schnorr_pks_list)
        .expect("Should be able to get schnorr_pks_list size");

    for i in 0..pks_list_size {

        let pk_object = _env.get_object_array_element(_schnorr_pks_list, i)
            .expect(format!("Should be able to get elem {} of schnorr_pks_list", i).as_str());

        let pk = _env.get_field(pk_object, "publicKeyPointer", "J")
            .expect("Should be able to get field publicKeyPointer");

        pks.push(*read_raw_pointer(pk.j().unwrap() as *const SchnorrPk));
    }

    //Extract threshold
    let threshold = _threshold as u64;

    //Compute constant
    match compute_pks_threshold_hash(pks.as_slice(), threshold) {
        Ok(constant) => return_field_element(&_env, constant),
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    }
}


#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_NaiveThresholdSigProof_nativeCreateMsgToSign(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _bt_list: jobjectArray,
    _epoch_number: jint,
    _end_cumulative_sc_tx_comm_tree_root: JObject,
    _btr_fee: jlong,
    _ft_min_fee: jlong,
) -> jobject
{
    //Extract backward transfers
    let mut bt_list = vec![];

    let bt_list_size = _env.get_array_length(_bt_list)
        .expect("Should be able to get bt_list size");

    if bt_list_size > 0
    {
        for i in 0..bt_list_size {
            let o = _env.get_object_array_element(_bt_list, i)
                .expect(format!("Should be able to get elem {} of bt_list array", i).as_str());

            let pk: [u8; MC_PK_SIZE] = {
                let p = _env.call_method(o, "getPublicKeyHash", "()[B", &[])
                    .expect("Should be able to call getPublicKeyHash method").l().unwrap().cast();

                let mut pk_bytes = [0u8; MC_PK_SIZE];

                _env.convert_byte_array(p)
                    .expect("Should be able to convert to Rust byte array")
                    .write(&mut pk_bytes[..])
                    .expect("Should be able to write into byte array of fixed size");

                pk_bytes
            };

            let a = _env.call_method(o, "getAmount", "()J", &[])
                .expect("Should be able to call getAmount method").j().unwrap() as u64;

            bt_list.push((a, pk));
        }
    }

    let end_cumulative_sc_tx_comm_tree_root = {
        let f =_env.get_field(_end_cumulative_sc_tx_comm_tree_root, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(f.j().unwrap() as *const FieldElement)
    };

    //Compute message to sign:
    let msg = match compute_msg_to_sign(
        _epoch_number as u32,
        &end_cumulative_sc_tx_comm_tree_root,
        _btr_fee as u64,
        _ft_min_fee as u64,
        bt_list
    ){
        Ok((_, msg)) => msg,
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR
    };

    //Return msg
    return_field_element(&_env, msg)
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_NaiveThresholdSigProof_nativeCreateProof(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _proving_system: JObject,
    _bt_list: jobjectArray,
    _epoch_number: jint,
    _end_cumulative_sc_tx_comm_tree_root: JObject,
    _btr_fee: jlong,
    _ft_min_fee: jlong,
    _schnorr_sigs_list: jobjectArray,
    _schnorr_pks_list:  jobjectArray,
    _threshold: jlong,
    _proving_key_path: JString,
    _check_proving_key: jboolean, //WARNING: Very expensive check
    _zk: jboolean,
) -> jobject
{
    // Extract proving system type
    let proving_system= _env
        .call_method(_proving_system, "ordinal", "()I", &[])
        .expect("Should be able to call ordinal() on ProvingSystem enum")
        .i()
        .unwrap() as usize;

    let proving_system = match proving_system {
        0 => ProvingSystem::Undefined,
        1 => ProvingSystem::Darlin,
        2 => ProvingSystem::CoboundaryMarlin,
        _ => unreachable!()
    };

    // Extract backward transfers
    let mut bt_list = vec![];

    let bt_list_size = _env.get_array_length(_bt_list)
        .expect("Should be able to get bt_list size");

    if bt_list_size > 0 {
        for i in 0..bt_list_size {
            let o = _env.get_object_array_element(_bt_list, i)
                .expect(format!("Should be able to get elem {} of bt_list array", i).as_str());


            let pk: [u8; MC_PK_SIZE] = {
                let p = _env.call_method(o, "getPublicKeyHash", "()[B", &[])
                    .expect("Should be able to call getPublicKeyHash method").l().unwrap().cast();

                let mut pk_bytes = [0u8; MC_PK_SIZE];

                _env.convert_byte_array(p)
                    .expect("Should be able to convert to Rust byte array")
                    .write(&mut pk_bytes[..])
                    .expect("Should be able to write into byte array of fixed size");

                pk_bytes
            };

            let a = _env.call_method(o, "getAmount", "()J", &[])
                .expect("Should be able to call getAmount method").j().unwrap() as u64;

            bt_list.push((a, pk));
        }
    }

    //Extract Schnorr signatures and the corresponding Schnorr pks
    let mut sigs = vec![];
    let mut pks = vec![];

    let sigs_list_size = _env.get_array_length(_schnorr_sigs_list)
        .expect("Should be able to get schnorr_sigs_list size");

    let pks_list_size = _env.get_array_length(_schnorr_pks_list)
        .expect("Should be able to get schnorr_pks_list size");

    assert_eq!(sigs_list_size, pks_list_size);

    for i in 0..sigs_list_size {
        //Get i-th sig
        let sig_object = _env.get_object_array_element(_schnorr_sigs_list, i)
            .expect(format!("Should be able to get elem {} of schnorr_sigs_list", i).as_str());

        let pk_object = _env.get_object_array_element(_schnorr_pks_list, i)
            .expect(format!("Should be able to get elem {} of schnorr_pks_list", i).as_str());

        let signature = {
            let sig = _env.get_field(sig_object, "signaturePointer", "J")
                .expect("Should be able to get field signaturePointer");

            match read_nullable_raw_pointer(sig.j().unwrap() as *const SchnorrSig) {
                Some(sig) => Some(*sig),
                None => None,
            }
        };

        let public_key = {
            let pk = _env.get_field(pk_object, "publicKeyPointer", "J")
                .expect("Should be able to get field publicKeyPointer");

            read_raw_pointer(pk.j().unwrap() as *const SchnorrPk)
        };

        sigs.push(signature);
        pks.push(*public_key);
    }

    let end_cumulative_sc_tx_comm_tree_root = {
        let f =_env.get_field(_end_cumulative_sc_tx_comm_tree_root, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(f.j().unwrap() as *const FieldElement)
    };

    //Extract params_path str
    let proving_key_path = _env.get_string(_proving_key_path)
        .expect("Should be able to read jstring as Rust String");

    //create proof
    let (proof, quality) = match create_naive_threshold_sig_proof(
        proving_system,
        pks.as_slice(),
        sigs,
        _epoch_number as u32,
        &end_cumulative_sc_tx_comm_tree_root,
        _btr_fee as u64,
        _ft_min_fee as u64,
        bt_list,
        _threshold as u64,
        proving_key_path.to_str().unwrap(),
        _check_proving_key == JNI_TRUE,
        _zk == JNI_TRUE,
    ) {
        Ok(proof) => proof,
        Err(_) => return std::ptr::null::<jobject>() as jobject //CRYPTO_ERROR or IO_ERROR
    };

    //Return proof serialized
    let proof_serialized = _env.byte_array_from_slice(proof.as_slice())
        .expect("Should be able to convert Rust slice into jbytearray");

    //Create new CreateProofResult object
    let proof_result_class = _env.find_class("com/horizen/sigproofnative/CreateProofResult")
        .expect("Should be able to find CreateProofResult class");

    let result = _env.new_object(
        proof_result_class,
        "([BJ)V",
        &[JValue::Object(JObject::from(proof_serialized)), JValue::Long(jlong::from(quality as i64))]
    ).expect("Should be able to create new CreateProofResult:(byte[], long) object");

    *result
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_provingsystemnative_ProvingSystem_nativeGenerateDLogKeys(
    _env: JNIEnv,
    _class: JClass,
    _proving_system: JObject,
    _segment_size: jint,
    _g1_key_path: JString,
    _g2_key_path: JString,
) -> jboolean
{
    // Extract proving system type
    let proving_system= _env
        .call_method(_proving_system, "ordinal", "()I", &[])
        .expect("Should be able to call ordinal() on ProvingSystem enum")
        .i()
        .unwrap() as usize;

    let proving_system = match proving_system {
        0 => ProvingSystem::Undefined,
        1 => ProvingSystem::Darlin,
        2 => ProvingSystem::CoboundaryMarlin,
        _ => unreachable!()
    };

    // Read paths
    let g1_key_path = _env.get_string(_g1_key_path)
        .expect("Should be able to read jstring as Rust String");

    let g2_key_path = _env.get_string(_g2_key_path)
        .expect("Should be able to read jstring as Rust String");

    // Generate DLOG keypair
    match init_dlog_keys(
        proving_system,
        _segment_size as usize,
        g1_key_path.to_str().unwrap(),
        g2_key_path.to_str().unwrap(),
    ) {
        Ok(_) => JNI_TRUE,
        Err(_) => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_NaiveThresholdSigProof_nativeSetup(
    _env: JNIEnv,
    _class: JClass,
    _proving_system: JObject,
    _max_pks: jlong,
    _proving_key_path: JString,
    _verification_key_path: JString,
) -> jboolean
{
    // Extract proving system type
    let proving_system= _env
        .call_method(_proving_system, "ordinal", "()I", &[])
        .expect("Should be able to call ordinal() on ProvingSystem enum")
        .i()
        .unwrap() as usize;

    let proving_system = match proving_system {
        0 => ProvingSystem::Undefined,
        1 => ProvingSystem::Darlin,
        2 => ProvingSystem::CoboundaryMarlin,
        _ => unreachable!()
    };

    // Read paths
    let proving_key_path = _env.get_string(_proving_key_path)
        .expect("Should be able to read jstring as Rust String");

    let verification_key_path = _env.get_string(_verification_key_path)
        .expect("Should be able to read jstring as Rust String");

    let max_pks = _max_pks as usize;

    let circ = get_instance_for_setup(max_pks);

    // Generate snark keypair
    match generate_circuit_keypair(
        circ,
        proving_system,
        proving_key_path.to_str().unwrap(),
        verification_key_path.to_str().unwrap()
    ) {
        Ok(_) => JNI_TRUE,
        Err(_) => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_horizen_sigproofnative_NaiveThresholdSigProof_nativeVerifyProof(
    _env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _class: JClass,
    _proving_system: JObject,
    _bt_list: jobjectArray,
    _epoch_number: jint,
    _end_cumulative_sc_tx_comm_tree_root: JObject,
    _btr_fee: jlong,
    _ft_min_fee: jlong,
    _constant: JObject,
    _quality: jlong,
    _sc_proof_bytes: jbyteArray,
    _check_proof: jboolean,
    _verification_key_path: JString,
    _check_vk: jboolean,
) -> jboolean
{
    // Extract proving system type
    let proving_system = _env
        .call_method(_proving_system, "ordinal", "()I", &[])
        .expect("Should be able to call ordinal() on ProvingSystem enum")
        .i()
        .unwrap() as usize;

    let proving_system = match proving_system {
        0 => ProvingSystem::Undefined,
        1 => ProvingSystem::Darlin,
        2 => ProvingSystem::CoboundaryMarlin,
        _ => unreachable!()
    };

    //Extract backward transfers
    let mut bt_list = vec![];

    let bt_list_size = _env.get_array_length(_bt_list)
        .expect("Should be able to get bt_list size");

    if bt_list_size > 0 {
        for i in 0..bt_list_size {
            let o = _env.get_object_array_element(_bt_list, i)
                .expect(format!("Should be able to get elem {} of bt_list array", i).as_str());


            let pk: [u8; MC_PK_SIZE] = {
                let p = _env.call_method(o, "getPublicKeyHash", "()[B", &[])
                    .expect("Should be able to call getPublicKeyHash method").l().unwrap().cast();

                let mut pk_bytes = [0u8; MC_PK_SIZE];

                _env.convert_byte_array(p)
                    .expect("Should be able to convert to Rust byte array")
                    .write(&mut pk_bytes[..])
                    .expect("Should be able to write into byte array of fixed size");

                pk_bytes
            };

            let a = _env.call_method(o, "getAmount", "()J", &[])
                .expect("Should be able to call getAmount method").j().unwrap() as u64;

            bt_list.push((a, pk));
        }
    }

    let end_cumulative_sc_tx_comm_tree_root = {
        let f =_env.get_field(_end_cumulative_sc_tx_comm_tree_root, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(f.j().unwrap() as *const FieldElement)
    };

    //Extract constant
    let constant = {

        let c =_env.get_field(_constant, "fieldElementPointer", "J")
            .expect("Should be able to get field fieldElementPointer");

        read_raw_pointer(c.j().unwrap() as *const FieldElement)
    };

    //Extract proof
    let proof_bytes = _env.convert_byte_array(_sc_proof_bytes)
        .expect("Should be able to convert to Rust byte array");

    //Extract vk path
    let vk_path = _env.get_string(_verification_key_path)
        .expect("Should be able to read jstring as Rust String");

    //Verify proof
    match verify_naive_threshold_sig_proof(
        proving_system,
        constant,
        _epoch_number as u32,
        end_cumulative_sc_tx_comm_tree_root,
        _btr_fee as u64,
        _ft_min_fee as u64,
        bt_list,
        _quality as u64,
        proof_bytes,
        _check_proof == JNI_TRUE,
        vk_path.to_str().unwrap(),
        _check_vk == JNI_TRUE,

    ) {
        Ok(result) => if result { JNI_TRUE } else { JNI_FALSE },
        Err(_) => JNI_FALSE // CRYPTO_ERROR or IO_ERROR
    }
}